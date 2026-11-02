use calamine::{CellErrorType, Data, Range};
use excel_model::{Cell, CellAddr, CellError, CellValue, Sheet};

/// Adds every non-empty value in a range to the sheet.
pub(super) fn add_values(sheet: &mut Sheet, range: &Range<Data>) {
    let Some((first_row, first_col)) = range.start() else {
        return;
    };
    for (row, col, data) in range.used_cells() {
        let value = to_value(data);
        if !value.is_empty() {
            let addr = CellAddr::new(first_row + row as u32, first_col + col as u32);
            sheet.set(addr, Some(Cell::literal(value)));
        }
    }
}

/// Converts a stored value. Dates become their serial numbers, because this
/// application has no date formatting.
fn to_value(data: &Data) -> CellValue {
    match data {
        Data::Empty => CellValue::Empty,
        Data::Int(n) => CellValue::Number(*n as f64),
        Data::Float(n) => CellValue::Number(*n),
        Data::String(text) | Data::DateTimeIso(text) | Data::DurationIso(text) => CellValue::Text(text.clone()),
        Data::Bool(b) => CellValue::Bool(*b),
        Data::DateTime(date) => CellValue::Number(date.as_f64()),
        Data::Error(error) => CellValue::Error(to_error(error)),
    }
}

fn to_error(error: &CellErrorType) -> CellError {
    match error {
        CellErrorType::Div0 => CellError::Div0,
        CellErrorType::NA | CellErrorType::GettingData => CellError::Na,
        CellErrorType::Name => CellError::Name,
        CellErrorType::Null => CellError::Null,
        CellErrorType::Num => CellError::Num,
        CellErrorType::Ref => CellError::Ref,
        CellErrorType::Value => CellError::Value,
    }
}
