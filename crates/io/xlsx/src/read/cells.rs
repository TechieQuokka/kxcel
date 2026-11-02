use calamine::{CellErrorType, Data, Range};
use excel_model::{Cell, CellAddr, CellError, CellValue, Sheet};

/// Adds every non-empty value in a range to the sheet.
pub(super) fn add_values(sheet: &mut Sheet, range: &Range<Data>) {
    let Some((first_row, first_col)) = range.start() else {
        return;
    };
    for (row, col, data) in range.used_cells() {
        let addr = CellAddr::new(first_row + row as u32, first_col + col as u32);
        if let Some(serial) = whole_date(data) {
            sheet.set(addr, Some(Cell::date(serial)));
            continue;
        }
        let value = to_value(data);
        if !value.is_empty() {
            sheet.set(addr, Some(Cell::literal(value)));
        }
    }
}

/// A date without a time of day, as its serial number. Dates with a time and
/// durations are read as plain numbers, since only whole dates are shown.
fn whole_date(data: &Data) -> Option<f64> {
    let Data::DateTime(date) = data else {
        return None;
    };
    let serial = date.as_f64();
    (date.is_datetime() && serial.fract() == 0.0 && excel_model::serial_date(serial).is_some()).then_some(serial)
}

/// Converts a stored value. Dates become their serial numbers.
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
