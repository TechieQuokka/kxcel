use egui::text::{CCursor, CCursorRange};
use egui::{Align, Frame, Id, Key, Layout, Modifiers, TextEdit, Ui, pos2};
use excel_command::CommandRegistry;

use crate::{CommandLineState, completion_popup, message};

const HINT: &str = "Press Ctrl+L for commands";

/// Something the user did in the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineAction {
    /// Run this line. The host decides where the keyboard goes next.
    Submit(String),
    /// Return to the grid.
    Leave,
    /// The user clicked into the command line.
    Focus,
}

/// Draws the command line and handles its input.
pub fn show(
    ui: &mut Ui,
    state: &mut CommandLineState,
    registry: &CommandRegistry,
    focused: bool,
) -> Option<CommandLineAction> {
    let id = Id::unique("excel_command_line");
    let mut action = None;

    if focused {
        // Requesting focus interrupts any input method composition, so only ask when needed.
        if !ui.memory(|memory| memory.has_focus(id)) {
            ui.memory_mut(|memory| memory.request_focus(id));
        }
        action = keys_before_text_input(ui, state);
    }

    if std::mem::take(&mut state.cursor_to_end) {
        let mut text_state = TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
        let end = CCursor::new(state.text.chars().count());
        text_state.cursor.set_char_range(Some(CCursorRange::one(end)));
        TextEdit::store_state(ui.ctx(), id, text_state);
    }

    let top_left = ui.max_rect().left_top();
    let mut before = None;
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.set_max_width(ui.available_width());
        let message_width = (ui.available_width() * 0.45).max(0.0);
        ui.allocate_ui(egui::vec2(message_width, ui.available_height()), |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| message::show(ui, state));
        });
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            before = Some(state.text.clone());
            let output = TextEdit::singleline(&mut state.text)
                .id(id)
                .frame(Frame::NONE.inner_margin(egui::Margin::symmetric(8, 4)))
                .font(egui::FontId::monospace(13.0))
                .hint_text(HINT)
                .desired_width(ui.available_width())
                .return_key(None)
                .show(ui);
            if !focused && output.response.clicked() {
                action = Some(CommandLineAction::Focus);
            }
        });
    });

    if before.as_deref() != Some(state.text.as_str()) {
        state.clear_message();
        state.picked = 0;
    }
    state.refresh(registry);

    if focused {
        if let Some(clicked) = completion_popup::show(ui, state, pos2(top_left.x + 4.0, top_left.y - 2.0)) {
            state.picked = clicked;
            state.complete();
        }
        // Read after the text box so that text an input method commits in the
        // same frame as Enter is part of the submitted line.
        if action.is_none() && ui.input(|i| i.key_pressed(Key::Enter)) {
            action = Some(CommandLineAction::Submit(state.take_line()));
        }
    }

    action
}

/// Handles keys the text box would otherwise act on.
fn keys_before_text_input(ui: &Ui, state: &mut CommandLineState) -> Option<CommandLineAction> {
    let has_candidates = !state.items.is_empty();
    ui.input_mut(|i| {
        if i.consume_key(Modifiers::NONE, Key::Escape) {
            state.take_line();
            return Some(CommandLineAction::Leave);
        }
        if has_candidates {
            if i.consume_key(Modifiers::NONE, Key::Tab) {
                state.complete();
            } else if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                state.pick_step(1);
            } else if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                state.pick_step(-1);
            }
        }
        None
    })
}
