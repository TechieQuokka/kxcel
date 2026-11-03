use excel_model::{CellAddr, CellValue};
use rust_xlsxwriter::{Format, Formula, Worksheet, XlsxError};

/// Writes a formula with its current value as the stored result, so that other
/// readers can show a value without recalculating.
pub(super) fn write_formula(
    target: &mut Worksheet,
    addr: CellAddr,
    text: &str,
    value: &CellValue,
    format: Option<&Format>,
) -> Result<(), XlsxError> {
    let result = match value {
        // Written at full precision, unlike the rounded form used for display.
        CellValue::Number(n) => n.to_string(),
        other => other.to_string(),
    };
    let formula = Formula::new(text).set_result(result);
    match format {
        Some(format) => target.write_formula_with_format(addr.row, addr.col as u16, formula, format)?,
        None => target.write_formula(addr.row, addr.col as u16, formula)?,
    };
    Ok(())
}
