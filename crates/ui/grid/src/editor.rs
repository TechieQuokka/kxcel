//! Keyboard handling and the in-cell editor.
//!
//! A text box stays focused at the cursor even when no edit is in progress,
//! at one pixel wide. The operating system only offers input methods to a
//! focused text box, so this is what lets the first key of a Korean word start
//! composing. Whatever arrives in the box starts an edit.
//!
//! The text box is multi-line, so Alt+Enter adds a line break. Tab is taken
//! before the text box, which would otherwise insert a tab character.
//!
//! Enter, Tab, and arrows are read after the text box has run. The input method
//! can deliver a commit and a key in the same frame, and this order keeps the
//! committed text in the cell being edited rather than the next one.

use egui::text::{CCursor, CCursorRange};
use egui::{Event, Frame, Id, Key, KeyboardShortcut, Modifiers, Rect, TextEdit, Ui, UiBuilder, vec2};
use excel_model::Sheet;

use crate::layout::{LINE_PADDING, Layout};
use crate::paint::CELL_PADDING;
use crate::{GridAction, GridState};

/// The text box's id, stable so focus and input method state survive cursor moves.
fn editor_id() -> Id {
    Id::unique("excel_grid_editor")
}

/// Handles shortcuts and clipboard events that must not reach the text box.
pub(crate) fn before_text_input(ui: &Ui, state: &mut GridState, actions: &mut Vec<GridAction>) {
    ui.input_mut(|i| {
        if i.consume_key(Modifiers::COMMAND, Key::L) {
            if state.editing {
                let addr = state.cursor;
                let text = state.take_text();
                actions.push(GridAction::Commit { addr, text });
            }
            actions.push(GridAction::FocusCommandLine);
        }
        if state.editing && !state.hints.is_empty() {
            if i.consume_key(Modifiers::NONE, Key::Tab) {
                state.accept_hint();
            } else if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                state.step_hint(1);
            } else if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                state.step_hint(-1);
            } else if i.consume_key(Modifiers::NONE, Key::Escape) {
                state.hints_dismissed = true;
                state.hints.clear();
            }
        }
        let shift = i.modifiers.shift;
        if i.consume_key(Modifiers::NONE, Key::Tab) || i.consume_key(Modifiers::SHIFT, Key::Tab) {
            state.pending_tab = Some(shift);
        }
        if state.editing {
            return;
        }
        // A line break only means something inside an edit.
        i.consume_key(Modifiers::ALT, Key::Enter);
        if i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z) || i.consume_key(Modifiers::COMMAND, Key::Y) {
            actions.push(GridAction::Redo);
        }
        if i.consume_key(Modifiers::COMMAND, Key::Z) {
            actions.push(GridAction::Undo);
        }
        let selection = state.selection();
        i.events.retain(|event| match event {
            Event::Copy => {
                actions.push(GridAction::Copy(selection));
                false
            }
            Event::Cut => {
                actions.push(GridAction::Cut(selection));
                false
            }
            Event::Paste(text) => {
                actions.push(GridAction::Paste { at: selection.start, text: text.clone() });
                false
            }
            _ => true,
        });
    });
}

/// Shows the text box. Returns true if it started an edit.
pub(crate) fn show(ui: &mut Ui, state: &mut GridState, layout: &Layout, line_height: f32) -> bool {
    let id = editor_id();
    let was_editing = state.editing;
    let cell = layout.cell_rect(state.cursor).unwrap_or_else(|| Rect::from_min_size(layout.body().min, vec2(1.0, 1.0)));

    let lines = state.text.split('\n').count();
    let multi_line = lines > 1;
    let rect = if was_editing {
        let wanted = state.text.split('\n').map(text_width).fold(0.0, f32::max) + 16.0;
        let right = (cell.left() + wanted.max(cell.width())).min(layout.body().right());
        let height = (lines as f32 * line_height + 2.0 * LINE_PADDING).max(cell.height());
        let bottom = (cell.top() + height).min(layout.body().bottom()).max(cell.bottom());
        Rect::from_min_max(cell.min, egui::pos2(right.max(cell.right()), bottom))
    } else {
        Rect::from_min_size(cell.min, vec2(1.0, cell.height()))
    };

    if state.cursor_to_end {
        state.cursor_to_end = false;
        let mut text_state = TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
        let end = CCursor::new(state.text.chars().count());
        text_state.cursor.set_char_range(Some(CCursorRange::one(end)));
        TextEdit::store_state(ui.ctx(), id, text_state);
    }

    // Requesting focus interrupts any input method composition, so only ask when needed.
    if !ui.memory(|memory| memory.has_focus(id)) {
        ui.memory_mut(|memory| memory.request_focus(id));
    }

    let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
    if was_editing {
        let fill = child.visuals().extreme_bg_color;
        let accent = child.visuals().selection.stroke.color;
        child.painter().rect_filled(rect, 0.0, fill);
        child.painter().rect_stroke(rect, 0.0, egui::Stroke::new(2.0, accent), egui::StrokeKind::Inside);
    } else {
        child.visuals_mut().text_edit_bg_color = Some(egui::Color32::TRANSPARENT);
        child.visuals_mut().text_cursor.stroke = egui::Stroke::NONE;
    }

    // The same position `paint` draws cell text at, so text stays put when editing starts.
    let (margin_y, align) =
        if multi_line { (LINE_PADDING, egui::Align2::LEFT_TOP) } else { (0.0, egui::Align2::LEFT_CENTER) };
    let mut edit = TextEdit::multiline(&mut state.text)
        .id(id)
        .frame(Frame::NONE.inner_margin(egui::Margin::symmetric(CELL_PADDING as i8, margin_y as i8)))
        .font(egui::FontId::monospace(crate::paint::CELL_FONT_SIZE))
        .desired_width(rect.width())
        .desired_rows(1)
        .min_size(rect.size())
        .align(align)
        .event_filter(egui::EventFilter { tab: false, horizontal_arrows: true, vertical_arrows: true, escape: true })
        .return_key(KeyboardShortcut::new(Modifiers::ALT, Key::Enter));
    if !was_editing {
        edit = edit.frame(Frame::NONE).desired_width(1.0);
    }
    let output = edit.show(&mut child);
    state.editor_text_rect = Some(Rect::from_min_size(output.galley_pos, output.galley.size()));

    let started = !was_editing && !state.text.is_empty();
    if started {
        state.editing = true;
        state.arrows_commit = true;
    }
    started
}

/// Handles keys that act on the finished text. See the module docs for why
/// this runs after the text box.
pub(crate) fn after_text_input(ui: &Ui, state: &mut GridState, sheet: &Sheet, actions: &mut Vec<GridAction>) {
    if state.editing {
        keys_while_editing(ui, state, actions);
    } else {
        keys_while_ready(ui, state, sheet, actions);
    }
}

fn keys_while_editing(ui: &Ui, state: &mut GridState, actions: &mut Vec<GridAction>) {
    let pending_tab = state.pending_tab.take();
    let (shift, enter, escape, arrow) = ui.input(|i| {
        let arrow = if i.key_pressed(Key::ArrowUp) {
            Some((-1, 0))
        } else if i.key_pressed(Key::ArrowDown) {
            Some((1, 0))
        } else if i.key_pressed(Key::ArrowLeft) {
            Some((0, -1))
        } else if i.key_pressed(Key::ArrowRight) {
            Some((0, 1))
        } else {
            None
        };
        // Alt+Enter is a line break inside the cell, not the end of the edit.
        let enter = i.key_pressed(Key::Enter) && !i.modifiers.alt;
        (i.modifiers.shift, enter, i.key_pressed(Key::Escape), arrow)
    });

    if escape {
        state.cancel_edit();
        return;
    }
    let step = if enter {
        Some((if shift { -1 } else { 1 }, 0))
    } else if let Some(back) = pending_tab {
        Some((0, if back { -1 } else { 1 }))
    } else if state.arrows_commit && !state.text.contains('\n') {
        arrow
    } else {
        None
    };
    if let Some((rows, cols)) = step {
        let addr = state.cursor;
        let text = state.take_text();
        actions.push(GridAction::Commit { addr, text });
        state.move_cursor(rows, cols, false);
    }
}

fn keys_while_ready(ui: &Ui, state: &mut GridState, sheet: &Sheet, actions: &mut Vec<GridAction>) {
    let page = i64::from(state.visible_rows.saturating_sub(2).max(1));
    if let Some(back) = state.pending_tab.take() {
        state.move_cursor(0, if back { -1 } else { 1 }, false);
    }
    ui.input(|i| {
        let shift = i.modifiers.shift;
        let command = i.modifiers.command;
        for (key, rows, cols) in [
            (Key::ArrowUp, -1, 0),
            (Key::ArrowDown, 1, 0),
            (Key::ArrowLeft, 0, -1),
            (Key::ArrowRight, 0, 1),
            (Key::PageUp, -page, 0),
            (Key::PageDown, page, 0),
        ] {
            if i.key_pressed(key) {
                state.move_cursor(rows, cols, shift);
            }
        }
        if i.key_pressed(Key::Enter) {
            state.move_cursor(if shift { -1 } else { 1 }, 0, false);
        }
        if i.key_pressed(Key::Home) {
            let rows = if command { -i64::from(state.cursor.row) } else { 0 };
            state.move_cursor(rows, -i64::from(state.cursor.col), shift);
        }
        if i.key_pressed(Key::F2) {
            let text = sheet.get(state.cursor).map(|cell| cell.input_text()).unwrap_or_default();
            state.begin_edit(text);
        }
        if i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace) {
            actions.push(GridAction::Clear(state.selection()));
        }
        if i.key_pressed(Key::Escape) {
            state.anchor = state.cursor;
        }
    });
}

/// A rough text width, enough to size the editor. Korean is about twice as wide.
fn text_width(text: &str) -> f32 {
    text.chars().map(|c| if c.is_ascii() { 8.0 } else { 15.0 }).sum()
}
