use excel_model::{CellAddr, CellValue};
use rust_xlsxwriter::{Formula, Worksheet, XlsxError};

/// Writes a literal value.
pub(super) fn write_value(target: &mut Worksheet, addr: CellAddr, value: &CellValue) -> Result<(), XlsxError> {
    let (row, col) = (addr.row, addr.col as u16);
    match value {
        CellValue::Empty => {}
        CellValue::Number(n) => {
            target.write_number(row, col, *n)?;
        }
        CellValue::Text(text) => {
            target.write_string(row, col, text)?;
        }
        CellValue::Bool(b) => {
            target.write_boolean(row, col, *b)?;
        }
        // The file format has no plain error value to write, so a literal
        // error is stored as a formula that evaluates to it.
        CellValue::Error(error) => {
            let text = error.to_string();
            target.write_formula(row, col, Formula::new(&text).set_result(&text))?;
        }
    }
    Ok(())
}
