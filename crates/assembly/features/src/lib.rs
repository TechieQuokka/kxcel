//! The one place features are registered.
//!
//! To add a command crate, add it to `Cargo.toml` and add one line to
//! [`registry`]. The GUI, the CLI, and completion all pick it up from here.

use std::path::{Path, PathBuf};

use excel_command::{Calculator, CommandRegistry, LoadedWorkbook, StoreError, WorkbookStore};
use excel_csv::CsvStore;
use excel_edit::Document;
use excel_formula_engine::FormulaEngine;
use excel_xlsx::XlsxStore;

/// The order command categories are listed in. Others follow.
pub const COMMAND_CATEGORIES: &[&str] = &["File", "Edit", "Navigate", "Sheet", "Help"];

/// The order function categories are listed in. Others follow.
pub const FUNCTION_CATEGORIES: &[&str] = &["Math", "Logic", "Text", "Lookup", "Date"];

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
    pub store: FileStore,
}

/// Picks the file format by extension: `.csv` files are CSV, and anything else
/// goes to `.xlsx`, which rejects what it cannot read.
#[derive(Debug, Default, Clone, Copy)]
pub struct FileStore;

impl FileStore {
    fn format(path: &Path) -> &'static dyn WorkbookStore {
        if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("csv")) { &CsvStore } else { &XlsxStore }
    }
}

impl WorkbookStore for FileStore {
    fn load(&self, path: &Path) -> Result<LoadedWorkbook, StoreError> {
        Self::format(path).load(path)
    }

    fn save(&self, workbook: &excel_model::Workbook, path: &Path) -> Result<(), StoreError> {
        Self::format(path).save(workbook, path)
    }

    fn save_losses(&self, workbook: &excel_model::Workbook, path: &Path) -> Vec<String> {
        Self::format(path).save_losses(workbook, path)
    }
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
        for name in [
            "help", "set", "save", "save!", "exit", "exit!", "open", "open!", "sort", "find", "goto", "insert",
            "delete", "sheet",
        ] {
            assert!(registry.complete(name).iter().any(|c| c.name == name), "{name}");
        }
    }

    #[test]
    fn csv_and_xlsx_files_round_trip_through_one_store() {
        let dir = tempfile::tempdir().unwrap();
        let mut book = excel_model::Workbook::new();
        book.active_mut().set("A1".parse().unwrap(), excel_model::Cell::parse_input("사과"));
        for name in ["a.csv", "a.CSV", "a.xlsx"] {
            let path = dir.path().join(name);
            FileStore.save(&book, &path).unwrap();
            let loaded = FileStore.load(&path).unwrap().workbook;
            assert_eq!(loaded.active().value("A1".parse().unwrap()).to_string(), "사과", "{name}");
        }
        assert!(FileStore.save(&book, &dir.path().join("a.txt")).is_err());
    }

    #[test]
    fn a_missing_file_opens_blank_with_its_path() {
        let doc = Services::default().open(Path::new("/nonexistent/new.xlsx")).unwrap();
        assert!(doc.workbook.active().is_empty());
        assert_eq!(doc.path, Some(PathBuf::from("/nonexistent/new.xlsx")));
    }
}
