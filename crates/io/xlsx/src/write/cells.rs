use excel_model::{CellAddr, CellValue, NumberFormat};
use rust_xlsxwriter::{Format, Formula, Worksheet, XlsxError};

/// The format a cell needs: a date format for dates, and unlocked for input
/// cells, which is how Excel marks cells a protected sheet lets users change.
/// `None` when the default will do.
pub(super) fn cell_format(format: NumberFormat, unlocked: bool) -> Option<Format> {
    let mut written = Format::new();
    if format == NumberFormat::Date {
        written = written.set_num_format("yyyy-mm-dd");
    }
    if unlocked {
        written = written.set_unlocked();
    }
    (format == NumberFormat::Date || unlocked).then_some(written)
}

/// Writes a literal value.
pub(super) fn write_value(
    target: &mut Worksheet,
    addr: CellAddr,
    value: &CellValue,
    format: Option<&Format>,
) -> Result<(), XlsxError> {
    let (row, col) = (addr.row, addr.col as u16);
    match (value, format) {
        (CellValue::Empty, Some(format)) => {
            target.write_blank(row, col, format)?;
        }
        (CellValue::Empty, None) => {}
        (CellValue::Number(n), Some(format)) => {
            target.write_number_with_format(row, col, *n, format)?;
        }
        (CellValue::Number(n), None) => {
            target.write_number(row, col, *n)?;
        }
        (CellValue::Text(text), Some(format)) => {
            target.write_string_with_format(row, col, text, format)?;
        }
        (CellValue::Text(text), None) => {
            target.write_string(row, col, text)?;
        }
        (CellValue::Bool(b), Some(format)) => {
            target.write_boolean_with_format(row, col, *b, format)?;
        }
        (CellValue::Bool(b), None) => {
            target.write_boolean(row, col, *b)?;
        }
        // The file format has no plain error value to write, so a literal
        // error is stored as a formula that evaluates to it.
        (CellValue::Error(error), format) => {
            let text = error.to_string();
            let formula = Formula::new(&text).set_result(&text);
            match format {
                Some(format) => target.write_formula_with_format(row, col, formula, format)?,
                None => target.write_formula(row, col, formula)?,
            };
        }
    }
    Ok(())
}
