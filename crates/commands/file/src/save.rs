use std::path::PathBuf;

use excel_command::path::expand_home;
use excel_command::{Command, CommandError, Confirmation, Context, quote_arg};
use excel_model::Doc;

pub(crate) struct Save {
    pub force: bool,
}

impl Command for Save {
    fn name(&self) -> &str {
        if self.force { "save!" } else { "save" }
    }

    fn doc(&self) -> Doc {
        if self.force {
            Doc {
                category: "File",
                summary: "Save without asking about content that will be lost",
                usage: "save! [file]",
                description: "Saves even when formatting, charts, or formulas would be lost.",
                args: &[("file", "Optional. Another file to save as")],
                related: &["save"],
                ..Doc::BLANK
            }
        } else {
            Doc {
                category: "File",
                summary: "Save, or save as another file",
                usage: "save [file]",
                description: "Saves to the current file, or to another file, which becomes the current one. \
                              A name without an extension gets .xlsx. Saving as .csv keeps only the active \
                              sheet's values. Asks first when something would be lost. Ctrl+S runs it.",
                args: &[("file", "Optional. An .xlsx or .csv file to save as")],
                examples: &[("save", ""), ("save report", ""), ("save values.csv", "")],
                related: &["save!", "open"],
            }
        }
    }

    fn takes_path(&self) -> bool {
        true
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        let target = match args {
            [] => ctx.doc.path.clone().ok_or_else(|| CommandError::new("No file name. Usage: save <file.xlsx>"))?,
            [path] => with_default_extension(expand_home(path)),
            _ => return Err(CommandError::new("Usage: save [file.xlsx]")),
        };

        if !self.force {
            // Saving elsewhere leaves the original intact, so what the opened file
            // has only matters when overwriting it. The format can lose more.
            let overwrites_source = ctx.doc.path.as_deref() == Some(target.as_path());
            let mut losses = if overwrites_source { ctx.doc.loss_warnings.clone() } else { Vec::new() };
            losses.extend(ctx.store.save_losses(&ctx.doc.workbook, &target));
            if !losses.is_empty() {
                let proceed = match args {
                    [] => "save!".to_owned(),
                    _ => format!("save! {}", quote_arg(&target.to_string_lossy())),
                };
                return Err(CommandError::confirm(
                    format!("Saving would lose: {}. Use {proceed} to save anyway", losses.join(", ")),
                    Confirmation::Loss { items: losses, proceed },
                ));
            }
        }

        ctx.store.save(&ctx.doc.workbook, &target)?;
        ctx.say(format!("Saved {}", target.display()));
        ctx.doc.path = Some(target);
        ctx.doc.loss_warnings.clear();
        ctx.doc.dirty = false;
        Ok(())
    }
}

/// Adds `.xlsx` to a name typed without an extension. Any extension typed is kept.
fn with_default_extension(mut path: PathBuf) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension("xlsx");
    }
    path
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{MemoryStore, NoCalculator, NoStore};
    use excel_edit::Document;
    use excel_model::Workbook;

    use super::*;

    const W: Save = Save { force: false };
    const FORCE: Save = Save { force: true };

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
        assert!(error.message.contains("charts") && error.message.contains("save!"), "{error}");
        assert_eq!(error.confirmation.as_ref().map(Confirmation::proceed), Some("save!"));
        assert!(store.files.borrow().is_empty());

        FORCE.run(&mut ctx, &[]).unwrap();
        assert!(store.files.borrow().contains_key(&PathBuf::from("a.xlsx")));
        assert!(doc.loss_warnings.is_empty());
    }

    #[test]
    fn a_name_without_an_extension_gets_xlsx() {
        let store = MemoryStore::default();
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &store);
        W.run(&mut ctx, &["report"]).unwrap();
        assert_eq!(doc.path, Some("report.xlsx".into()));
        let mut ctx = Context::new(&mut doc, &NoCalculator, &store);
        W.run(&mut ctx, &["report.v2.xlsx"]).unwrap();
        assert_eq!(doc.path, Some("report.v2.xlsx".into()));
        assert_eq!(with_default_extension("data.csv".into()), PathBuf::from("data.csv"));
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
    fn a_format_that_loses_content_asks_first() {
        let mut store = MemoryStore::default();
        store.save_losses.insert("csv".into(), vec!["formulas".into()]);
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &store);
        let error = W.run(&mut ctx, &["my out.csv"]).unwrap_err();
        assert_eq!(error.confirmation.as_ref().map(Confirmation::proceed), Some(r#"save! "my out.csv""#));
        assert!(store.files.borrow().is_empty());
        FORCE.run(&mut ctx, &["my out.csv"]).unwrap();
        assert!(store.files.borrow().contains_key(&PathBuf::from("my out.csv")));
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
