use std::fs;
use std::path::{Path, PathBuf};

use excel_command::StoreError;
use excel_model::{CellInput, Workbook};
use rust_xlsxwriter::Workbook as XlsxWorkbook;

use super::{cells, formulas};
use crate::error::{file_error, require_xlsx};

/// Writes a workbook. The file is replaced in one step, so a failure partway
/// through leaves any existing file intact.
pub fn write(workbook: &Workbook, path: &Path) -> Result<(), StoreError> {
    require_xlsx(path)?;
    let bytes = encode(workbook).map_err(|e| file_error("save", path, e))?;
    let temporary = temporary_path(path);
    fs::write(&temporary, bytes).map_err(|e| file_error("save", path, e))?;
    fs::rename(&temporary, path).map_err(|e| {
        let _ = fs::remove_file(&temporary);
        file_error("save", path, e)
    })
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

fn encode(workbook: &Workbook) -> Result<Vec<u8>, rust_xlsxwriter::XlsxError> {
    let mut output = XlsxWorkbook::new();
    for sheet in workbook.sheets() {
        let target = output.add_worksheet();
        target.set_name(&sheet.name)?;
        for (col, width) in sheet.custom_col_widths() {
            target.set_column_width_pixels(col as u16, width.round() as u32)?;
        }
        for (addr, cell) in sheet.iter() {
            match &cell.input {
                CellInput::Literal(value) => cells::write_value(target, addr, value, cell.format)?,
                CellInput::Formula(text) => formulas::write_formula(target, addr, text, &cell.value, cell.format)?,
            }
        }
    }
    output.save_to_buffer()
}
