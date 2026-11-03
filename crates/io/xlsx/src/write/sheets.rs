use std::fs;
use std::path::{Path, PathBuf};

use excel_command::StoreError;
use excel_model::{CellAddr, CellInput, MAX_ROWS, Sheet, Table, Workbook, header_names};
use rust_xlsxwriter::{Format, Table as XlsxTable, TableColumn, Workbook as XlsxWorkbook, Worksheet};

use super::{cells, formulas};
use crate::error::{file_error, require_xlsx};

/// Writes a workbook. The file is replaced in one step, so a failure partway
/// through leaves any existing file intact.
pub fn write(workbook: &Workbook, path: &Path) -> Result<(), StoreError> {
    require_xlsx(path)?;
    let bytes = encode(workbook).map_err(|e| file_error("save", path, e))?;
    let bytes = crate::tables::add_filters(bytes, workbook).map_err(|e| file_error("save", path, e))?;
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
            let format = cells::cell_format(cell.format, sheet.is_input(addr));
            match &cell.input {
                CellInput::Literal(value) => cells::write_value(target, addr, value, format.as_ref())?,
                CellInput::Formula(text) => formulas::write_formula(target, addr, text, &cell.value, format.as_ref())?,
            }
        }
        write_empty_inputs(target, sheet)?;
        for row in sheet.hidden_rows().iter() {
            target.set_row_hidden(row)?;
        }
        for table in sheet.tables() {
            write_table(target, sheet, table)?;
        }
    }
    output.save_to_buffer()
}

/// Marks empty input cells unlocked. An input range covering whole columns
/// is written as a column format rather than a million blank cells.
fn write_empty_inputs(target: &mut Worksheet, sheet: &Sheet) -> Result<(), rust_xlsxwriter::XlsxError> {
    let unlocked = Format::new().set_unlocked();
    for range in sheet.meta().input_cells.ranges() {
        if range.start.row == 0 && range.end.row == MAX_ROWS - 1 {
            for col in range.start.col..=range.end.col {
                target.set_column_format(col as u16, &unlocked)?;
            }
            continue;
        }
        for addr in range.iter().filter(|addr| sheet.get(*addr).is_none()) {
            target.write_blank(addr.row, addr.col as u16, &unlocked)?;
        }
    }
    Ok(())
}

/// Writes a table with a header row and filter buttons. Its headers are
/// written as the names they show, made unique as Excel requires. Excel needs
/// a row below the header, so a table that is only a header gains an empty one.
fn write_table(target: &mut Worksheet, sheet: &Sheet, table: &Table) -> Result<(), rust_xlsxwriter::XlsxError> {
    let range = table.range;
    let shown: Vec<String> =
        (range.start.col..=range.end.col).map(|col| sheet.display(CellAddr::new(range.start.row, col))).collect();
    let columns: Vec<TableColumn> =
        header_names(&shown).into_iter().map(|name| TableColumn::new().set_header(name)).collect();
    let output = XlsxTable::new().set_name(&table.name).set_header_row(true).set_autofilter(true).set_columns(&columns);
    let last_row = range.end.row.max(range.start.row + 1);
    target.add_table(range.start.row, range.start.col as u16, last_row, range.end.col as u16, &output)?;
    Ok(())
}
