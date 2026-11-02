use excel_model::{CellAddr, CellValue, NumberFormat};
use rust_xlsxwriter::{Formula, Worksheet, XlsxError};

/// Writes a formula with its current value as the stored result, so that other
/// readers can show a value without recalculating.
pub(super) fn write_formula(
    target: &mut Worksheet,
    addr: CellAddr,
    text: &str,
    value: &CellValue,
    format: NumberFormat,
) -> Result<(), XlsxError> {
    let result = match value {
        // Written at full precision, unlike the rounded form used for display.
        CellValue::Number(n) => n.to_string(),
        other => other.to_string(),
    };
    let formula = Formula::new(text).set_result(result);
    match format {
        NumberFormat::Date => {
            target.write_formula_with_format(addr.row, addr.col as u16, formula, &super::cells::date_format())?
        }
        NumberFormat::General => target.write_formula(addr.row, addr.col as u16, formula)?,
    };
    Ok(())
}
