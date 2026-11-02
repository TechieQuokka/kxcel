use std::path::Path;

use excel_command::{LoadedWorkbook, StoreError, WorkbookStore};
use excel_model::Workbook;

use crate::{inspect, read, write};

/// Stores workbooks as `.xlsx` files.
#[derive(Debug, Default, Clone, Copy)]
pub struct XlsxStore;

impl WorkbookStore for XlsxStore {
    fn load(&self, path: &Path) -> Result<LoadedWorkbook, StoreError> {
        let workbook = read(path)?;
        let loss_warnings = inspect(path)?;
        Ok(LoadedWorkbook { workbook, loss_warnings })
    }

    fn save(&self, workbook: &Workbook, path: &Path) -> Result<(), StoreError> {
        write(workbook, path)
    }
}
