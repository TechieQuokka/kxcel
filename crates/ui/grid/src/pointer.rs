use egui::{CursorIcon, PointerButton, Pos2, Rect, Response, Ui};
use excel_model::{CellAddr, CellRange, Sheet};

use crate::layout::{Layout, ROW_HEIGHT};
use crate::paint::{fill_handle_rect, scroll_extent, track_rects};
use crate::state::Drag;
use crate::{GridAction, GridState, accepts_reference, autofit, fill_target, shrunk_away};

const MIN_COL_WIDTH: f32 = 24.0;
/// How many pixels of horizontal scrolling move one column.
const COL_SCROLL_STEP: f32 = 96.0;

/// Handles clicks, drags, and scrolling.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle(
    ui: &Ui,
    response: &Response,
    state: &mut GridState,
    sheet: &Sheet,
    layout: &Layout,
    whole: Rect,
    focused: bool,
    actions: &mut Vec<GridAction>,
) {
    if response.hovered() {
        wheel(ui, state, sheet);
    }

    let hover = ui.input(|i| i.pointer.hover_pos());
    if let Some(pos) = hover {
        if state.drag.is_none() && layout.resize_handle_at(pos).is_some() {
            ui.set_cursor_icon(CursorIcon::ResizeHorizontal);
        } else if state.drag.is_none() && hovered_fill_handle(state, layout, pos) {
            ui.set_cursor_icon(CursorIcon::Crosshair);
        }
    }

    let pressed = ui.input(|i| i.pointer.button_pressed(PointerButton::Primary));
    let press_pos = ui.input(|i| i.pointer.press_origin());
    if pressed
        && let Some(pos) = press_pos
        && whole.contains(pos)
        && response.contains_pointer()
    {
        press(ui, state, sheet, layout, whole, pos, focused, actions);
    }

    // Double-clicking a column edge in the header fits the column to its contents.
    if response.double_clicked()
        && let Some(pos) = response.interact_pointer_pos()
        && let Some(span) = layout.resize_handle_at(pos)
    {
        actions.push(GridAction::ResizeColumn { col: span.col, width: autofit::fitted_width(ui, sheet, span.col) });
        state.drag = None;
    }

    if response.double_clicked()
        && let Some(pos) = response.interact_pointer_pos()
        && let Some(addr) = layout.cell_at(pos)
        && !state.is_editing()
    {
        state.cursor = addr;
        state.anchor = addr;
        let text = sheet.get(addr).map(|cell| cell.input_text()).unwrap_or_default();
        state.begin_edit(text);
        state.drag = None;
    }

    if let Some(drag) = state.drag {
        let pos = ui.input(|i| i.pointer.latest_pos());
        if let Some(pos) = pos {
            continue_drag(drag, state, sheet, layout, whole, pos, actions);
        }
        if !ui.input(|i| i.pointer.primary_down()) {
            finish_drag(ui, state, actions);
        }
    }
}

fn wheel(ui: &Ui, state: &mut GridState, sheet: &Sheet) {
    let delta = ui.input(|i| i.smooth_scroll_delta());
    if delta == egui::Vec2::ZERO {
        return;
    }
    state.scroll_remainder += delta;
    let rows = (state.scroll_remainder.y / ROW_HEIGHT).trunc();
    let cols = (state.scroll_remainder.x / COL_SCROLL_STEP).trunc();
    state.scroll_remainder.y -= rows * ROW_HEIGHT;
    state.scroll_remainder.x -= cols * COL_SCROLL_STEP;
    state.scroll_by(-(rows as i64), -(cols as i64), sheet.hidden_rows());
}

#[allow(clippy::too_many_arguments)]
fn press(
    ui: &Ui,
    state: &mut GridState,
    sheet: &Sheet,
    layout: &Layout,
    whole: Rect,
    pos: Pos2,
    focused: bool,
    actions: &mut Vec<GridAction>,
) {
    if !focused {
        actions.push(GridAction::FocusGrid);
    }

    let (vertical, horizontal) = track_rects(whole, state.header_size().y);
    if vertical.contains(pos) {
        state.drag = Some(Drag::ScrollVertical);
        return;
    }
    if horizontal.contains(pos) {
        state.drag = Some(Drag::ScrollHorizontal);
        return;
    }
    if let Some(span) = layout.resize_handle_at(pos) {
        state.drag = Some(Drag::Resize { col: span.col, start_x: pos.x, start_width: span.width });
        return;
    }
    if !state.is_editing() && hovered_fill_handle(state, layout, pos) {
        let source = state.selection();
        state.drag = Some(Drag::Fill { source, target: source });
        return;
    }

    let Some(addr) = layout.cell_at(pos) else {
        return;
    };

    if !state.is_editing()
        && !state.app_mode
        && on_header_mark(sheet, layout, addr, pos)
        && let Some(popup) = crate::FilterPopup::open(sheet, addr)
    {
        state.cursor = addr;
        state.anchor = addr;
        state.popup = Some(popup);
        return;
    }

    if state.is_editing() {
        if addr == state.cursor && !accepts_reference(&state.text) {
            return;
        }
        if accepts_reference(&state.text) {
            let insert_at = state.text.len();
            state.text.push_str(&addr.to_string());
            state.cursor_to_end = true;
            state.reference_end = addr;
            state.drag = Some(Drag::FormulaRange { start: addr, insert_at });
            return;
        }
        let addr_before = state.cursor;
        let text = state.take_text();
        actions.push(GridAction::Commit { addr: addr_before, text });
    }

    state.cursor = addr;
    if !ui.input(|i| i.modifiers.shift) {
        state.anchor = addr;
    }
    state.drag = Some(Drag::Select);
}

fn continue_drag(
    drag: Drag,
    state: &mut GridState,
    sheet: &Sheet,
    layout: &Layout,
    whole: Rect,
    pos: Pos2,
    actions: &mut Vec<GridAction>,
) {
    match drag {
        Drag::Select => {
            if let Some(addr) = drag_cell(state, sheet, layout, pos) {
                state.cursor = addr;
            }
        }
        Drag::Fill { source, .. } => {
            if let Some(addr) = drag_cell(state, sheet, layout, pos) {
                state.drag = Some(Drag::Fill { source, target: fill_target(source, addr) });
            }
        }
        Drag::Resize { col, start_x, start_width } => {
            let width = (start_width + pos.x - start_x).max(MIN_COL_WIDTH);
            actions.push(GridAction::ResizeColumn { col, width });
        }
        Drag::FormulaRange { start, insert_at } => {
            if let Some(addr) = drag_cell(state, sheet, layout, pos) {
                state.text.truncate(insert_at);
                state.text.push_str(&CellRange::new(start, addr).to_string());
                state.cursor_to_end = true;
                state.reference_end = addr;
            }
        }
        Drag::ScrollVertical => {
            let (track, _) = track_rects(whole, state.header_size().y);
            let (rows, _) = scroll_extent(state, sheet);
            let ratio = ((pos.y - track.top()) / track.height()).clamp(0.0, 1.0);
            state.top_row = (ratio * rows.saturating_sub(state.visible_rows) as f32) as u32;
        }
        Drag::ScrollHorizontal => {
            let (_, track) = track_rects(whole, state.header_size().y);
            let (_, cols) = scroll_extent(state, sheet);
            let ratio = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
            state.left_col = (ratio * cols.saturating_sub(state.visible_cols) as f32) as u32;
        }
    }
}

/// The cell under the pointer during a drag. Near an edge, the view scrolls by
/// one step so that a selection can grow past what is visible.
fn drag_cell(state: &mut GridState, sheet: &Sheet, layout: &Layout, pos: Pos2) -> Option<CellAddr> {
    let body = layout.body();
    let rows = if pos.y >= body.bottom() {
        1
    } else if pos.y < body.top() {
        -1
    } else {
        0
    };
    let cols = if pos.x >= body.right() {
        1
    } else if pos.x < body.left() {
        -1
    } else {
        0
    };
    if rows != 0 || cols != 0 {
        state.scroll_by(rows, cols, sheet.hidden_rows());
    }
    layout.nearest_cell(pos)
}

fn finish_drag(ui: &Ui, state: &mut GridState, actions: &mut Vec<GridAction>) {
    if let Some(Drag::Fill { source, target }) = state.drag
        && target != source
    {
        match shrunk_away(source, target) {
            Some(cleared) => actions.push(GridAction::Clear(cleared)),
            None => {
                let alternate = ui.input(|i| i.modifiers.command);
                actions.push(GridAction::Fill { source, target, alternate });
            }
        }
        state.anchor = target.start;
        state.cursor = target.end;
    }
    state.drag = None;
}

/// Whether `pos` is on the `▼` at the right of a table header cell.
fn on_header_mark(sheet: &Sheet, layout: &Layout, addr: CellAddr, pos: Pos2) -> bool {
    let header = sheet.table_at(addr).is_some_and(|table| table.is_header(addr));
    header && layout.cell_rect(addr).is_some_and(|rect| pos.x >= rect.right() - crate::paint::HEADER_MARK_WIDTH)
}

fn hovered_fill_handle(state: &GridState, layout: &Layout, pos: Pos2) -> bool {
    !state.is_editing()
        && !state.app_mode
        && fill_handle_rect(layout, state.selection()).is_some_and(|rect| rect.expand(2.0).contains(pos))
}
