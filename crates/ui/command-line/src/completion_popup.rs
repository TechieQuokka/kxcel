use egui::{Align2, Area, Frame, Id, Order, Pos2, RichText, ScrollArea, Ui};

use crate::CommandLineState;
use crate::state::Completing;

/// The tallest the list grows before it scrolls.
const MAX_HEIGHT: f32 = 320.0;

/// Shows choices above the command line. Returns a clicked choice's index.
pub(crate) fn show(ui: &Ui, state: &mut CommandLineState, anchor: Pos2) -> Option<usize> {
    if state.items.is_empty() {
        return None;
    }
    let mut clicked = None;
    let scroll = std::mem::take(&mut state.scroll_to_pick);
    Area::new(Id::unique("excel_command_completion"))
        .order(Order::Foreground)
        .fixed_pos(anchor)
        .pivot(Align2::LEFT_BOTTOM)
        .show(ui.ctx(), |ui| {
            Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(360.0);
                ui.spacing_mut().item_spacing.y = 2.0;
                let title = match state.completing {
                    Completing::Commands => "Commands",
                    Completing::Paths => "Files",
                };
                ui.label(RichText::new(title).small().weak());
                let width = state.items.iter().map(|item| item.label.chars().count()).max().unwrap_or(0) + 2;
                ScrollArea::vertical().max_height(MAX_HEIGHT).show(ui, |ui| {
                    for (index, item) in state.items.iter().enumerate() {
                        let text = format!("{:<width$}{}", item.label, item.help);
                        let selected = index == state.picked;
                        let response = ui.selectable_label(selected, RichText::new(text).monospace());
                        if selected && scroll {
                            response.scroll_to_me(None);
                        }
                        if response.clicked() {
                            clicked = Some(index);
                        }
                    }
                });
            });
        });
    clicked
}
