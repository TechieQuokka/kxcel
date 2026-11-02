//! The one place features are registered.
//!
//! To add a command crate, add it to `Cargo.toml` and add one line to
//! [`registry`]. The GUI, the CLI, and completion all pick it up from here.

use std::path::{Path, PathBuf};

use excel_command::{Calculator, CommandRegistry, StoreError, WorkbookStore};
use excel_edit::Document;
use excel_formula_engine::FormulaEngine;
use excel_xlsx::XlsxStore;

/// Every available command.
pub fn registry() -> CommandRegistry {
    let mut registry = CommandRegistry::default();
    excel_cmd_cell::register(&mut registry);
    excel_cmd_file::register(&mut registry);
    excel_cmd_help::register(&mut registry);
    excel_cmd_sort::register(&mut registry);
    excel_cmd_find::register(&mut registry);
    excel_cmd_navigate::register(&mut registry);
    excel_cmd_structure::register(&mut registry);
    excel_cmd_sheet::register(&mut registry);
    registry
}

/// The real services commands run with.
#[derive(Default)]
pub struct Services {
    pub calculator: FormulaEngine,
    pub store: XlsxStore,
}

impl Services {
    /// Opens a file, calculated and ready to show. A path that does not exist
    /// gives a blank document that will save there.
    pub fn open(&self, path: &Path) -> Result<Document, StoreError> {
        if !path.exists() {
            return Ok(Document::new(Default::default(), Some(PathBuf::from(path)), Vec::new()));
        }
        let mut loaded = self.store.load(path)?;
        self.calculator.recalculate(&mut loaded.workbook);
        Ok(Document::new(loaded.workbook, Some(PathBuf::from(path)), loaded.loss_warnings))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_is_registered() {
        let registry = registry();
        for name in
            ["help", "set", "w", "w!", "q", "q!", "e", "e!", "sort", "find", "goto", "insert", "delete", "sheet"]
        {
            assert!(registry.complete(name).iter().any(|c| c.name == name), "{name}");
        }
    }

    #[test]
    fn a_missing_file_opens_blank_with_its_path() {
        let doc = Services::default().open(Path::new("/nonexistent/new.xlsx")).unwrap();
        assert!(doc.workbook.active().is_empty());
        assert_eq!(doc.path, Some(PathBuf::from("/nonexistent/new.xlsx")));
    }
}
