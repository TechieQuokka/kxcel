use egui::{Align2, Color32, FontId, Rect, Stroke, StrokeKind, Ui, pos2, vec2};
use excel_model::{Cell, CellRange, CellValue, MAX_COLS, MAX_ROWS, Sheet};

use crate::GridState;
use crate::layout::{COL_HEADER_HEIGHT, LINE_PADDING, Layout, SCROLLBAR_SIZE};
use crate::state::Drag;

pub(crate) const CELL_PADDING: f32 = 6.0;
pub(crate) const CELL_FONT_SIZE: f32 = 13.0;
pub(crate) const FILL_HANDLE_SIZE: f32 = 7.0;
/// Extra rows and columns the scrollbars allow beyond the used area.
const SCROLL_MARGIN: u32 = 50;

/// Draws cells, grid lines, the selection, and the fill handle.
pub(crate) fn cells(ui: &Ui, state: &GridState, sheet: &Sheet, layout: &Layout) {
    let visuals = ui.visuals();
    let body = layout.body();
    let painter = ui.painter().with_clip_rect(body);
    let line = Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color);
    let font = FontId::monospace(CELL_FONT_SIZE);
    let accent = visuals.selection.stroke.color;
    let selection = state.selection();

    painter.rect_filled(body, 0.0, visuals.extreme_bg_color);

    for span in &layout.cols {
        for row_span in &layout.rows {
            let addr = excel_model::CellAddr::new(row_span.row, span.col);
            let rect = Rect::from_min_size(pos2(span.x, row_span.y), vec2(span.width, row_span.height));
            if selection.contains(addr) {
                painter.rect_filled(rect, 0.0, visuals.selection.bg_fill.gamma_multiply(0.45));
            }
            if let Some(cell) = sheet.get(addr) {
                paint_cell_text(ui, cell, rect, &font);
            }
        }
    }

    for row_span in &layout.rows {
        let y = (row_span.y + row_span.height).round() - 0.5;
        painter.hline(body.x_range(), y, line);
    }
    for span in &layout.cols {
        let x = (span.x + span.width).round() - 0.5;
        painter.vline(x, body.y_range(), line);
    }

    match state.drag {
        Some(Drag::Fill { source, target }) => {
            if let Some(cleared) = crate::shrunk_away(source, target) {
                shade(ui, layout, cleared);
            }
            outline(ui, layout, target, accent);
        }
        Some(Drag::FormulaRange { start, .. }) => {
            outline(ui, layout, CellRange::new(start, state.reference_end), accent);
        }
        _ => {}
    }

    if let Some(rect) = layout.cell_rect(state.cursor) {
        painter.rect_stroke(rect, 0.0, Stroke::new(2.0, accent), StrokeKind::Inside);
    }
    if !state.is_editing()
        && let Some(rect) = fill_handle_rect(layout, selection)
    {
        painter.rect_filled(rect, 0.0, accent);
    }
}

fn paint_cell_text(ui: &Ui, cell: &Cell, rect: Rect, font: &FontId) {
    let mut text = cell.value.to_string();
    if text.is_empty() {
        if !cell.uncalculated {
            return;
        }
        // A formula that could not be calculated and has no stored result.
        text.push('?');
    }
    let visuals = ui.visuals();
    let color = match &cell.value {
        CellValue::Error(_) => visuals.error_fg_color,
        _ if cell.uncalculated => visuals.warn_fg_color,
        _ => visuals.text_color(),
    };
    let numeric = matches!(cell.value, CellValue::Number(_));
    let (pos, anchor) = if text.contains('\n') {
        // Multi-line text starts at the top, where the editor puts it too.
        (pos2(rect.left() + CELL_PADDING, rect.top() + LINE_PADDING), Align2::LEFT_TOP)
    } else if numeric {
        (pos2(rect.right() - CELL_PADDING, rect.center().y), Align2::RIGHT_CENTER)
    } else {
        (pos2(rect.left() + CELL_PADDING, rect.center().y), Align2::LEFT_CENTER)
    };
    ui.painter().with_clip_rect(rect.shrink(1.0).intersect(ui.clip_rect())).text(
        pos,
        anchor,
        text,
        font.clone(),
        color,
    );
}

/// Greys out cells a shrinking fill is about to clear.
fn shade(ui: &Ui, layout: &Layout, range: CellRange) {
    let (Some(a), Some(b)) = (visible_corner(layout, range, true), visible_corner(layout, range, false)) else {
        return;
    };
    let color = ui.visuals().widgets.inactive.bg_fill.gamma_multiply(0.8);
    ui.painter().with_clip_rect(layout.body()).rect_filled(a.union(b), 0.0, color);
}

fn outline(ui: &Ui, layout: &Layout, range: CellRange, color: Color32) {
    let (Some(a), Some(b)) = (visible_corner(layout, range, true), visible_corner(layout, range, false)) else {
        return;
    };
    let painter = ui.painter().with_clip_rect(layout.body());
    painter.rect_stroke(a.union(b), 0.0, Stroke::new(1.5, color), StrokeKind::Inside);
}

/// The rectangle of a range's top-left or bottom-right cell, clamped into view.
fn visible_corner(layout: &Layout, range: CellRange, top_left: bool) -> Option<Rect> {
    let first = layout.cols.first()?;
    let last = layout.cols.last()?;
    let last_row = layout.rows.last()?.row;
    let row = if top_left { range.start.row } else { range.end.row }.clamp(layout.top_row, last_row);
    let col = if top_left { range.start.col } else { range.end.col }.clamp(first.col, last.col);
    layout.cell_rect(excel_model::CellAddr::new(row, col))
}

/// Where the fill handle is, if the selection's corner is in view.
pub(crate) fn fill_handle_rect(layout: &Layout, selection: CellRange) -> Option<Rect> {
    let corner = layout.cell_rect(selection.end)?;
    Some(Rect::from_center_size(corner.max - vec2(2.0, 2.0), vec2(FILL_HANDLE_SIZE, FILL_HANDLE_SIZE)))
}

/// The scrollbar tracks of the whole grid area.
pub(crate) fn track_rects(rect: Rect) -> (Rect, Rect) {
    let vertical = Rect::from_min_max(
        pos2(rect.right() - SCROLLBAR_SIZE, rect.top() + COL_HEADER_HEIGHT),
        pos2(rect.right(), rect.bottom() - SCROLLBAR_SIZE),
    );
    let horizontal = Rect::from_min_max(
        pos2(rect.left(), rect.bottom() - SCROLLBAR_SIZE),
        pos2(rect.right() - SCROLLBAR_SIZE, rect.bottom()),
    );
    (vertical, horizontal)
}

/// How many rows and columns the scrollbars span: enough for the used area,
/// the cursor, and the current scroll position, plus a margin to keep scrolling.
pub(crate) fn scroll_extent(state: &GridState, sheet: &Sheet) -> (u32, u32) {
    let (used_rows, used_cols) = sheet.extent();
    let rows = used_rows
        .max(state.cursor.row + 1)
        .max(state.top_row + state.visible_rows)
        .saturating_add(SCROLL_MARGIN)
        .min(MAX_ROWS);
    let cols = used_cols
        .max(state.cursor.col + 1)
        .max(state.left_col + state.visible_cols)
        .saturating_add(SCROLL_MARGIN)
        .min(MAX_COLS);
    (rows, cols)
}

pub(crate) fn scrollbars(ui: &Ui, state: &GridState, sheet: &Sheet, rect: Rect) {
    let visuals = ui.visuals();
    let painter = ui.painter();
    let thumb_color = visuals.widgets.inactive.bg_fill;
    let (vertical, horizontal) = track_rects(rect);
    let (rows, cols) = scroll_extent(state, sheet);
    painter.rect_filled(vertical, 0.0, visuals.faint_bg_color);
    painter.rect_filled(horizontal, 0.0, visuals.faint_bg_color);

    let (start, length) = thumb(state.top_row, state.visible_rows, rows, vertical.height());
    let thumb_v =
        Rect::from_min_size(pos2(vertical.left() + 2.0, vertical.top() + start), vec2(SCROLLBAR_SIZE - 4.0, length));
    painter.rect_filled(thumb_v, 3.0, thumb_color);

    let (start, length) = thumb(state.left_col, state.visible_cols, cols, horizontal.width());
    let thumb_h = Rect::from_min_size(
        pos2(horizontal.left() + start, horizontal.top() + 2.0),
        vec2(length, SCROLLBAR_SIZE - 4.0),
    );
    painter.rect_filled(thumb_h, 3.0, thumb_color);
}

/// A scrollbar thumb's start and length along its track.
fn thumb(position: u32, visible: u32, total: u32, track: f32) -> (f32, f32) {
    let total = total.max(1) as f32;
    let length = (visible as f32 / total * track).clamp(20.0_f32.min(track), track);
    let start = position as f32 / total * (track - length).max(0.0);
    (start, length)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_has_a_minimum_length() {
        let (_, length) = thumb(0, 10, 1_000_000, 500.0);
        assert_eq!(length, 20.0);
    }

    #[test]
    fn thumb_moves_with_position() {
        assert_eq!(thumb(0, 10, 100, 500.0).0, 0.0);
        assert!(thumb(50, 10, 100, 500.0).0 > 200.0);
    }

    #[test]
    fn scroll_extent_covers_used_cells_and_cursor() {
        let mut sheet = Sheet::new("S");
        sheet.set("C200".parse().unwrap(), Some(Cell::literal(CellValue::Number(1.0))));
        let mut state = GridState::default();
        state.set_visible(10, 5);
        assert_eq!(scroll_extent(&state, &sheet), (250, 55));
        state.cursor = "A1000".parse().unwrap();
        assert_eq!(scroll_extent(&state, &sheet).0, 1050);
    }
}
