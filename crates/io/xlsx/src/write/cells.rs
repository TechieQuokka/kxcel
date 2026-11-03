use excel_model::{Cell, CellAddr, CellValue, NumberFormat, format_number};
use rust_xlsxwriter::{Format, Formula, Worksheet, XlsxError};

/// The format a cell needs: a date format for dates, a comma format for
/// numbers of four digits or more, a plain number format (`0`) for cells
/// marked plain, and unlocked for input cells, which is how Excel marks cells
/// a protected sheet lets users change. `None` when the default will do. An
/// empty cell is given as `None`.
pub(super) fn cell_format(cell: Option<&Cell>, plain: bool, unlocked: bool) -> Option<Format> {
    let number = cell.and_then(|cell| match cell.value {
        CellValue::Number(n) if cell.format == NumberFormat::General => Some(n),
        _ => None,
    });
    let code = match (cell.map(|cell| cell.format), number) {
        (Some(NumberFormat::Date), _) => Some("yyyy-mm-dd".to_owned()),
        (_, Some(n)) if plain => Some(number_code("0", n)),
        (_, Some(n)) if n.abs() >= 1000.0 => Some(number_code("#,##0", n)).filter(|code| !code.contains('E')),
        _ if plain => Some("0".to_owned()),
        _ => None,
    };
    if code.is_none() && !unlocked {
        return None;
    }
    let mut written = Format::new();
    if let Some(code) = code {
        written = written.set_num_format(code);
    }
    if unlocked {
        written = written.set_unlocked();
    }
    Some(written)
}

/// A number format showing `n` with exactly the decimals it shows here, so
/// Excel shows the same digits: `#,##0` or `0`, then `.00` and so on.
fn number_code(whole: &str, n: f64) -> String {
    let text = format_number(n);
    if text.contains('E') {
        return "E".to_owned();
    }
    match text.split_once('.') {
        Some((_, fraction)) => format!("{whole}.{}", "0".repeat(fraction.len())),
        None => whole.to_owned(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn code(text: &str, plain: bool) -> Option<String> {
        cell_format(Cell::parse_input(text).as_ref(), plain, false).map(|_| number_code_of(text, plain))
    }

    /// The code `cell_format` picks, worked out the same way for checking.
    fn number_code_of(text: &str, plain: bool) -> String {
        let cell = Cell::parse_input(text).unwrap();
        match cell.value {
            CellValue::Number(n) => number_code(if plain { "0" } else { "#,##0" }, n),
            _ => "0".into(),
        }
    }

    #[test]
    fn picks_comma_and_plain_codes() {
        assert_eq!(number_code("#,##0", 1234567.0), "#,##0");
        assert_eq!(number_code("#,##0", 1234.25), "#,##0.00");
        assert_eq!(number_code("0", 2026.0), "0");
        assert!(code("999", false).is_none(), "small numbers look the same either way");
        assert!(code("1234", false).is_some());
        assert!(code("abc", false).is_none());
        assert!(code("abc", true).is_some(), "plain is kept on any cell");
        assert!(cell_format(None, true, false).is_some() && cell_format(None, false, false).is_none());
    }
}
