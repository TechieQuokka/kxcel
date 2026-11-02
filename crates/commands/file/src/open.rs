use std::path::PathBuf;

use excel_command::{Command, CommandError, Context, Effect};
use excel_edit::Document;

pub(crate) struct Open {
    pub force: bool,
}

impl Command for Open {
    fn name(&self) -> &str {
        if self.force { "e!" } else { "e" }
    }

    fn help(&self) -> &str {
        if self.force { "Open a file, discarding unsaved changes" } else { "Open a file: e data.xlsx" }
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        let [path] = args else {
            return Err(CommandError::new("Usage: e <file.xlsx>"));
        };
        if ctx.doc.dirty && !self.force {
            return Err(CommandError::new("Unsaved changes. Use w to save, or e! to discard them"));
        }

        let path = PathBuf::from(path);
        let mut loaded = ctx.store.load(&path)?;
        ctx.calculator.recalculate(&mut loaded.workbook);

        let warning = (!loaded.loss_warnings.is_empty()).then(|| {
            format!("Opened {}. Saving over it would lose: {}", path.display(), loaded.loss_warnings.join(", "))
        });
        ctx.say(warning.unwrap_or_else(|| format!("Opened {}", path.display())));
        *ctx.doc = Document::new(loaded.workbook, Some(path), loaded.loss_warnings);
        ctx.effects.push(Effect::DocumentReplaced);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{MemoryStore, NoCalculator, NoStore};
    use excel_model::{Sheet, Workbook};

    use super::*;

    const E: Open = Open { force: false };

    fn store_with_file() -> MemoryStore {
        let store = MemoryStore::default();
        let book = Workbook::from_sheets(vec![Sheet::new("Loaded")]);
        store.files.borrow_mut().insert("x.xlsx".into(), book);
        store
    }

    #[test]
    fn replaces_the_document() {
        let store = store_with_file();
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &store);
        E.run(&mut ctx, &["x.xlsx"]).unwrap();
        assert_eq!(ctx.effects, [Effect::DocumentReplaced]);
        assert_eq!(doc.workbook.active().name, "Loaded");
        assert_eq!(doc.path, Some("x.xlsx".into()));
    }

    #[test]
    fn refuses_with_unsaved_changes_unless_forced() {
        let store = store_with_file();
        let mut doc = Document { dirty: true, ..Default::default() };
        let mut ctx = Context::new(&mut doc, &NoCalculator, &store);
        assert!(E.run(&mut ctx, &["x.xlsx"]).is_err());
        Open { force: true }.run(&mut ctx, &["x.xlsx"]).unwrap();
        assert!(!doc.dirty);
    }

    #[test]
    fn a_failed_open_keeps_the_document() {
        let mut doc = Document::default();
        doc.workbook.active_mut().name = "Mine".into();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        assert!(E.run(&mut ctx, &["missing.xlsx"]).is_err());
        assert_eq!(doc.workbook.active().name, "Mine");
    }

    #[test]
    fn reports_what_saving_would_lose() {
        let mut store = store_with_file();
        store.loss_warnings = vec!["charts".into()];
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &store);
        E.run(&mut ctx, &["x.xlsx"]).unwrap();
        assert!(ctx.message.as_deref().unwrap().contains("charts"));
        assert_eq!(doc.loss_warnings, ["charts"]);
    }
}
