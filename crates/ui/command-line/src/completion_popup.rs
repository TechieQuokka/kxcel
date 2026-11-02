use egui::{Align2, Area, Frame, Id, Order, Pos2, RichText, Ui};

use crate::CommandLineState;

/// Shows candidates above the command line. Returns a clicked candidate's index.
pub(crate) fn show(ui: &Ui, state: &CommandLineState, anchor: Pos2) -> Option<usize> {
    if state.candidates.is_empty() {
        return None;
    }
    let mut clicked = None;
    Area::new(Id::unique("excel_command_completion"))
        .order(Order::Foreground)
        .fixed_pos(anchor)
        .pivot(Align2::LEFT_BOTTOM)
        .show(ui.ctx(), |ui| {
            Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(360.0);
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new("Commands").small().weak());
                for (index, candidate) in state.candidates.iter().enumerate() {
                    let text = format!("{:<10}{}", candidate.name, candidate.help);
                    let selected = index == state.picked;
                    if ui.selectable_label(selected, RichText::new(text).monospace()).clicked() {
                        clicked = Some(index);
                    }
                }
            });
        });
    clicked
}
