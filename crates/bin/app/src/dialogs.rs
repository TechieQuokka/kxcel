use egui::{Context, Id, Key, Modal};

/// A question that must be answered before continuing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    /// The opened file has content that saving over it would lose.
    LossWarning(Vec<String>),
    /// The window is closing with unsaved changes.
    UnsavedChanges,
}

/// What the user chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Dismiss,
    SaveAndClose,
    DiscardAndClose,
}

/// Shows a dialog. Returns the user's choice once they make one.
pub fn show(ctx: &Context, dialog: &Dialog) -> Option<Choice> {
    let mut choice = None;
    Modal::new(Id::unique("excel_dialog")).show(ctx, |ui| {
        ui.set_max_width(420.0);
        match dialog {
            Dialog::LossWarning(items) => {
                ui.heading("This file has content that will not be saved");
                ui.add_space(6.0);
                ui.label(format!("Saving over this file would lose: {}.", items.join(", ")));
                ui.label("Use w <new file> to keep the original, or w! to overwrite it.");
                ui.add_space(10.0);
                if ui.button("OK").clicked() {
                    choice = Some(Choice::Dismiss);
                }
            }
            Dialog::UnsavedChanges => {
                ui.heading("Save changes before closing?");
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        choice = Some(Choice::SaveAndClose);
                    }
                    if ui.button("Don't save").clicked() {
                        choice = Some(Choice::DiscardAndClose);
                    }
                    if ui.button("Cancel").clicked() {
                        choice = Some(Choice::Dismiss);
                    }
                });
            }
        }
    });
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        choice = Some(Choice::Dismiss);
    }
    choice
}
