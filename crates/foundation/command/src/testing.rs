//! Fake services, so a feature crate can be tested without the real ones.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use excel_model::Workbook;

use crate::{Calculator, LoadedWorkbook, StoreError, Structural, WorkbookStore};

/// A calculator that computes nothing and leaves formulas unchanged.
pub struct NoCalculator;

impl Calculator for NoCalculator {
    fn recalculate(&self, _workbook: &mut Workbook) {}

    fn translate(&self, formula: &str, _rows: i64, _cols: i64) -> String {
        formula.to_owned()
    }

    fn adjust(&self, formula: &str, _own: &str, _changed: &str, _change: Structural) -> String {
        formula.to_owned()
    }

    fn rename_sheet(&self, formula: &str, _old: &str, _new: &str) -> String {
        formula.to_owned()
    }

    fn remove_sheet(&self, formula: &str, _removed: &str) -> String {
        formula.to_owned()
    }
}

/// A store with no files. Every operation fails.
pub struct NoStore;

impl WorkbookStore for NoStore {
    fn load(&self, path: &Path) -> Result<LoadedWorkbook, StoreError> {
        Err(StoreError(format!("No such file: {}", path.display())))
    }

    fn save(&self, _workbook: &Workbook, path: &Path) -> Result<(), StoreError> {
        Err(StoreError(format!("Cannot save: {}", path.display())))
    }
}

/// A store that keeps workbooks in memory.
#[derive(Default)]
pub struct MemoryStore {
    pub files: RefCell<HashMap<PathBuf, Workbook>>,
    /// Warnings returned with every load.
    pub loss_warnings: Vec<String>,
    /// What saving to a path with this extension leaves out.
    pub save_losses: HashMap<String, Vec<String>>,
}

impl WorkbookStore for MemoryStore {
    fn load(&self, path: &Path) -> Result<LoadedWorkbook, StoreError> {
        let workbook = self.files.borrow().get(path).cloned();
        workbook
            .map(|workbook| LoadedWorkbook { workbook, loss_warnings: self.loss_warnings.clone() })
            .ok_or_else(|| StoreError(format!("No such file: {}", path.display())))
    }

    fn save(&self, workbook: &Workbook, path: &Path) -> Result<(), StoreError> {
        self.files.borrow_mut().insert(path.to_owned(), workbook.clone());
        Ok(())
    }

    fn save_losses(&self, _workbook: &Workbook, path: &Path) -> Vec<String> {
        let extension = path.extension().map(|ext| ext.to_string_lossy().into_owned()).unwrap_or_default();
        self.save_losses.get(&extension).cloned().unwrap_or_default()
    }
}
