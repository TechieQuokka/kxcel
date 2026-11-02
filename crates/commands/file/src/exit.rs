use excel_command::{Command, CommandError, Confirmation, Context, Effect};
use excel_model::Doc;

pub(crate) struct Exit {
    pub force: bool,
}

impl Command for Exit {
    fn name(&self) -> &str {
        if self.force { "exit!" } else { "exit" }
    }

    fn doc(&self) -> Doc {
        if self.force {
            Doc {
                category: "File",
                summary: "Exit, discarding unsaved changes",
                usage: "exit!",
                description: "Exits without asking, even with unsaved changes.",
                related: &["exit"],
                ..Doc::BLANK
            }
        } else {
            Doc {
                category: "File",
                summary: "Exit the application",
                usage: "exit",
                description: "With unsaved changes, asks whether to save first. Closing the window does the \
                              same. In kxcel-cli, stops running the remaining commands.",
                related: &["exit!", "save"],
                ..Doc::BLANK
            }
        }
    }

    fn run(&self, ctx: &mut Context<'_>, _args: &[&str]) -> Result<(), CommandError> {
        if ctx.doc.dirty && !self.force {
            return Err(CommandError::confirm(
                "Unsaved changes. Use save to keep them, or exit! to discard them",
                Confirmation::UnsavedChanges { action: "closing".into(), proceed: "exit!".into() },
            ));
        }
        ctx.effects.push(Effect::Quit);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{NoCalculator, NoStore};
    use excel_edit::Document;

    use super::*;

    #[test]
    fn exits_when_clean() {
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        Exit { force: false }.run(&mut ctx, &[]).unwrap();
        assert_eq!(ctx.effects, [Effect::Quit]);
    }

    #[test]
    fn asks_to_confirm_unsaved_changes_unless_forced() {
        let mut doc = Document { dirty: true, ..Default::default() };
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        let error = Exit { force: false }.run(&mut ctx, &[]).unwrap_err();
        assert!(error.message.contains("exit!"), "{error}");
        assert!(
            matches!(error.confirmation, Some(Confirmation::UnsavedChanges { ref proceed, .. }) if proceed == "exit!")
        );
        assert!(ctx.effects.is_empty());
        Exit { force: true }.run(&mut ctx, &[]).unwrap();
        assert_eq!(ctx.effects, [Effect::Quit]);
    }
}
