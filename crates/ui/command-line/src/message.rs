use egui::{RichText, Ui};

use crate::CommandLineState;

/// Shows the latest command result.
pub(crate) fn show(ui: &mut Ui, state: &CommandLineState) {
    if let Some((text, is_error)) = &state.message {
        let mut label = RichText::new(text).monospace();
        label = if *is_error { label.color(ui.visuals().error_fg_color) } else { label.weak() };
        ui.add(egui::Label::new(label).truncate());
    }
}
