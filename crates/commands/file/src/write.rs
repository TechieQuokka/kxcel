use std::path::PathBuf;

use excel_command::{Command, CommandError, Context};

pub(crate) struct Write {
    pub force: bool,
}

impl Command for Write {
    fn name(&self) -> &str {
        if self.force { "w!" } else { "w" }
    }

    fn help(&self) -> &str {
        if self.force {
            "Save, losing formatting and charts the file has"
        } else {
            "Save, optionally as a new file: w out.xlsx"
        }
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        let target = match args {
            [] => ctx.doc.path.clone().ok_or_else(|| CommandError::new("No file name. Usage: w <file.xlsx>"))?,
            [path] => PathBuf::from(path),
            _ => return Err(CommandError::new("Usage: w [file.xlsx]")),
        };

        // Saving elsewhere leaves the original intact, so only overwriting it needs confirmation.
        let overwrites_source = ctx.doc.path.as_deref() == Some(target.as_path());
        if overwrites_source && !ctx.doc.loss_warnings.is_empty() && !self.force {
            return Err(CommandError::new(format!(
                "Saving would lose: {}. Use w! to save anyway, or w <new file>",
                ctx.doc.loss_warnings.join(", ")
            )));
        }

        ctx.store.save(&ctx.doc.workbook, &target)?;
        ctx.say(format!("Saved {}", target.display()));
        ctx.doc.path = Some(target);
        ctx.doc.loss_warnings.clear();
        ctx.doc.dirty = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{MemoryStore, NoCalculator, NoStore};
    use excel_edit::Document;
    use excel_model::Workbook;

    use super::*;

    const W: Write = Write { force: false };
    const FORCE: Write = Write { force: true };

    fn doc_with_warning() -> Document {
        Document::new(Workbook::new(), Some("a.xlsx".into()), vec!["charts".into()])
    }

    #[test]
    fn saves_to_the_current_path_and_clears_dirty() {
        let store = MemoryStore::default();
        let mut doc = Document::new(Workbook::new(), Some("a.xlsx".into()), Vec::new());
        doc.dirty = true;
        let mut ctx = Context::new(&mut doc, &NoCalculator, &store);
        W.run(&mut ctx, &[]).unwrap();
        assert_eq!(ctx.message.as_deref(), Some("Saved a.xlsx"));
        assert!(!doc.dirty);
        assert!(store.files.borrow().contains_key(&PathBuf::from("a.xlsx")));
    }

    #[test]
    fn needs_a_name_for_a_new_document() {
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        assert!(W.run(&mut ctx, &[]).is_err());
    }

    #[test]
    fn save_as_changes_the_path() {
        let store = MemoryStore::default();
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &store);
        W.run(&mut ctx, &["b.xlsx"]).unwrap();
        assert_eq!(doc.path, Some("b.xlsx".into()));
    }

    #[test]
    fn overwriting_a_file_with_lossy_content_needs_confirmation() {
        let store = MemoryStore::default();
        let mut doc = doc_with_warning();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &store);
        let error = W.run(&mut ctx, &[]).unwrap_err();
        assert!(error.0.contains("charts") && error.0.contains("w!"), "{error}");
        assert!(store.files.borrow().is_empty());

        FORCE.run(&mut ctx, &[]).unwrap();
        assert!(store.files.borrow().contains_key(&PathBuf::from("a.xlsx")));
        assert!(doc.loss_warnings.is_empty());
    }

    #[test]
    fn saving_elsewhere_needs_no_confirmation() {
        let store = MemoryStore::default();
        let mut doc = doc_with_warning();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &store);
        W.run(&mut ctx, &["copy.xlsx"]).unwrap();
        assert_eq!(doc.path, Some("copy.xlsx".into()));
    }

    #[test]
    fn a_failed_save_stays_dirty() {
        let mut doc = Document::new(Workbook::new(), Some("a.xlsx".into()), Vec::new());
        doc.dirty = true;
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        assert!(W.run(&mut ctx, &[]).is_err());
        assert!(doc.dirty);
    }
}
