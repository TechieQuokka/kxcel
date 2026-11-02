use egui::{Context, Id, Key, Modal};
use excel_command::Confirmation;

/// A question that must be answered before continuing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    /// The opened file has content that saving over it would lose.
    LossWarning(Vec<String>),
    /// A command stopped to ask first. `then` holds the command lines to run
    /// after it goes ahead, such as `exit!` after a save.
    Confirm { confirmation: Confirmation, then: Vec<String> },
}

/// What the user chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Close the dialog and do nothing.
    Dismiss,
    /// Save first, then go ahead.
    Save,
    /// Go ahead without saving.
    Proceed,
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
                ui.label("Use save <new file> to keep the original.");
                ui.add_space(10.0);
                if ui.button("OK").clicked() {
                    choice = Some(Choice::Dismiss);
                }
            }
            Dialog::Confirm { confirmation: Confirmation::UnsavedChanges { action, .. }, .. } => {
                ui.heading(format!("Save changes before {action}?"));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        choice = Some(Choice::Save);
                    }
                    if ui.button("Don't save").clicked() {
                        choice = Some(Choice::Proceed);
                    }
                    if ui.button("Cancel").clicked() {
                        choice = Some(Choice::Dismiss);
                    }
                });
            }
            Dialog::Confirm { confirmation: Confirmation::Ask { question, yes, .. }, .. } => {
                ui.heading(question);
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(yes).clicked() {
                        choice = Some(Choice::Proceed);
                    }
                    if ui.button("Cancel").clicked() {
                        choice = Some(Choice::Dismiss);
                    }
                });
            }
            Dialog::Confirm { confirmation: Confirmation::Loss { items, .. }, .. } => {
                ui.heading("Some content will not be saved");
                ui.add_space(6.0);
                ui.label(format!("Saving would lose: {}.", items.join(", ")));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Save anyway").clicked() {
                        choice = Some(Choice::Proceed);
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

/// The command lines a choice runs, in order.
pub fn lines_for(dialog: &Dialog, choice: Choice) -> Vec<String> {
    let Dialog::Confirm { confirmation, then } = dialog else {
        return Vec::new();
    };
    let mut lines = match choice {
        Choice::Dismiss => return Vec::new(),
        Choice::Save => vec!["save".to_owned(), confirmation.proceed().to_owned()],
        Choice::Proceed => vec![confirmation.proceed().to_owned()],
    };
    lines.extend(then.iter().cloned());
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unsaved() -> Dialog {
        Dialog::Confirm {
            confirmation: Confirmation::UnsavedChanges { action: "closing".into(), proceed: "exit!".into() },
            then: Vec::new(),
        }
    }

    #[test]
    fn saving_first_then_goes_ahead() {
        assert_eq!(lines_for(&unsaved(), Choice::Save), ["save", "exit!"]);
        assert_eq!(lines_for(&unsaved(), Choice::Proceed), ["exit!"]);
        assert!(lines_for(&unsaved(), Choice::Dismiss).is_empty());
    }

    #[test]
    fn a_nested_question_keeps_what_comes_after() {
        let loss = Dialog::Confirm {
            confirmation: Confirmation::Loss { items: vec!["charts".into()], proceed: "save!".into() },
            then: vec!["exit!".into()],
        };
        assert_eq!(lines_for(&loss, Choice::Proceed), ["save!", "exit!"]);
        assert!(lines_for(&Dialog::LossWarning(Vec::new()), Choice::Proceed).is_empty());
    }
}
