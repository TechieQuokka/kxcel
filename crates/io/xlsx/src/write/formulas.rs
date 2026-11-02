use excel_model::{CellAddr, CellValue};
use rust_xlsxwriter::{Formula, Worksheet, XlsxError};

/// Writes a formula with its current value as the stored result, so that other
/// readers can show a value without recalculating.
pub(super) fn write_formula(
    target: &mut Worksheet,
    addr: CellAddr,
    text: &str,
    value: &CellValue,
) -> Result<(), XlsxError> {
    let result = match value {
        // Written at full precision, unlike the rounded form used for display.
        CellValue::Number(n) => n.to_string(),
        other => other.to_string(),
    };
    target.write_formula(addr.row, addr.col as u16, Formula::new(text).set_result(result))?;
    Ok(())
}
