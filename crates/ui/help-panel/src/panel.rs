use egui::{Frame, Id, Key, Margin, Modifiers, RichText, ScrollArea, TextEdit, Ui};

use crate::{HelpSection, HelpState, filter};

/// Something the user did in the help panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpAction {
    /// The user clicked into the search box.
    FocusSearch,
    /// The user pressed Escape in the search box.
    Leave,
    /// The user closed the panel.
    Close,
}

const DEFAULT_WIDTH: f32 = 340.0;

/// Draws the panel on the right if help is open. `search_focused` says whether
/// the search box should have keyboard focus.
pub fn show(ui: &mut Ui, state: &mut HelpState, sections: &[HelpSection], search_focused: bool) -> Option<HelpAction> {
    if !state.open {
        state.search_rect = None;
        return None;
    }
    let id = Id::unique("excel_help_search");
    let mut action = None;

    if search_focused {
        // Requesting focus interrupts input method composition, so only ask when needed.
        if !ui.memory(|memory| memory.has_focus(id)) {
            ui.memory_mut(|memory| memory.request_focus(id));
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            action = Some(HelpAction::Leave);
        }
    }

    let fill = ui.visuals().panel_fill;
    egui::Panel::right("excel_help_panel")
        .default_size(DEFAULT_WIDTH)
        .resizable(true)
        .frame(Frame::NONE.fill(fill).inner_margin(Margin::same(10)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Help").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("x").on_hover_text("Close (F1)").clicked() {
                        action = Some(HelpAction::Close);
                    }
                });
            });
            ui.add_space(6.0);

            let search = TextEdit::singleline(&mut state.query)
                .id(id)
                .hint_text("Search")
                .desired_width(f32::INFINITY)
                .show(ui)
                .response;
            state.search_rect = Some(search.rect);
            if !search_focused && search.clicked() {
                action = Some(HelpAction::FocusSearch);
            }
            ui.add_space(8.0);

            let found = filter(sections, &state.query);
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                if found.is_empty() {
                    ui.label(RichText::new("No matches").weak());
                }
                for section in &found {
                    ui.label(RichText::new(&section.title).strong());
                    ui.add_space(2.0);
                    let color = section.color.unwrap_or(ui.visuals().selection.stroke.color);
                    for entry in &section.entries {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(&entry.name).monospace().color(color));
                            ui.label(RichText::new(&entry.usage).weak());
                        });
                    }
                    ui.add_space(10.0);
                }
            });
        });

    action
}
