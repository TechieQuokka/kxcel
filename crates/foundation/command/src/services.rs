use std::fmt;
use std::path::Path;

use excel_model::Workbook;

/// Inserted or deleted rows or columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Structural {
    InsertRows { at: u32, count: u32 },
    DeleteRows { at: u32, count: u32 },
    InsertCols { at: u32, count: u32 },
    DeleteCols { at: u32, count: u32 },
}

/// Formula operations. Implemented by the formula engine and supplied at runtime,
/// so that commands do not depend on it directly.
pub trait Calculator {
    /// Recomputes every formula in the workbook.
    fn recalculate(&self, workbook: &mut Workbook);

    /// Rewrites a formula for a copy moved by an offset. Relative references move
    /// with it, and references that leave the sheet become `#REF!`.
    fn translate(&self, formula: &str, rows: i64, cols: i64) -> String;

    /// Rewrites a formula living on `own_sheet` after `change` was made to `changed_sheet`.
    fn adjust(&self, formula: &str, own_sheet: &str, changed_sheet: &str, change: Structural) -> String;
}

/// A workbook read from a file.
#[derive(Debug)]
pub struct LoadedWorkbook {
    pub workbook: Workbook,
    /// Things in the file that saving would lose, such as formatting.
    pub loss_warnings: Vec<String>,
}

/// Why a file could not be read or written. The text is shown to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError(pub String);

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for StoreError {}

/// File operations. Implemented by the file crate and supplied at runtime.
pub trait WorkbookStore {
    fn load(&self, path: &Path) -> Result<LoadedWorkbook, StoreError>;
    fn save(&self, workbook: &Workbook, path: &Path) -> Result<(), StoreError>;
}
