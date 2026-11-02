use excel_command::{Command, CommandError, Context, Effect};

pub(crate) struct Quit {
    pub force: bool,
}

impl Command for Quit {
    fn name(&self) -> &str {
        if self.force { "q!" } else { "q" }
    }

    fn help(&self) -> &str {
        if self.force { "Quit, discarding unsaved changes" } else { "Quit" }
    }

    fn run(&self, ctx: &mut Context<'_>, _args: &[&str]) -> Result<(), CommandError> {
        if ctx.doc.dirty && !self.force {
            return Err(CommandError::new("Unsaved changes. Use w to save, or q! to discard them"));
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
    fn quits_when_clean() {
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        Quit { force: false }.run(&mut ctx, &[]).unwrap();
        assert_eq!(ctx.effects, [Effect::Quit]);
    }

    #[test]
    fn refuses_with_unsaved_changes_unless_forced() {
        let mut doc = Document { dirty: true, ..Default::default() };
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        assert!(Quit { force: false }.run(&mut ctx, &[]).is_err());
        assert!(ctx.effects.is_empty());
        Quit { force: true }.run(&mut ctx, &[]).unwrap();
        assert_eq!(ctx.effects, [Effect::Quit]);
    }
}
