use std::path::Path;

use calamine::{Reader, Xlsx, open_workbook};
use excel_command::StoreError;
use excel_model::{Sheet, Workbook};

use super::{cells, formulas};
use crate::error::{file_error, require_xlsx};

/// Reads a workbook, showing the sheet it was saved showing. Formulas keep the
/// results stored in the file, so a caller should recalculate before showing it.
pub fn read(path: &Path) -> Result<Workbook, StoreError> {
    require_xlsx(path)?;
    let mut file: Xlsx<_> = open_workbook(path).map_err(|e| file_error("open", path, e))?;

    let mut sheets = Vec::new();
    for name in file.sheet_names() {
        let mut sheet = Sheet::new(name.clone());
        let values = file.worksheet_range(&name).map_err(|e| file_error("read", path, e))?;
        cells::add_values(&mut sheet, &values);
        let formula_range = file.worksheet_formula(&name).map_err(|e| file_error("read", path, e))?;
        formulas::add_formulas(&mut sheet, &formula_range);
        sheets.push(sheet);
    }
    let archive = std::fs::File::open(path).map_err(|e| file_error("open", path, e))?;
    let mut archive = zip::ZipArchive::new(archive).map_err(|e| file_error("read", path, e))?;
    let mut meta = crate::tables::read_meta(&mut archive).map_err(|e| file_error("read", path, e))?;
    for sheet in &mut sheets {
        if let Some(found) = meta.sheets.remove(&sheet.name) {
            sheet.set_meta(found);
        }
    }
    let active = meta.active_sheet.and_then(|name| sheets.iter().position(|sheet| sheet.name == name));
    let mut workbook = Workbook::from_sheets(sheets);
    workbook.set_active(active.unwrap_or(0));
    Ok(workbook)
}
