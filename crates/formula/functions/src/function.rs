use excel_model::{CellError, CellValue};

/// A function argument.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    /// A typed value, such as `5`, `"x"`, or the result of an expression.
    Value(CellValue),
    /// Values read from cells. Empty cells are not included.
    Cells(Vec<CellValue>),
}

/// A spreadsheet function. Arguments are already evaluated, and errors arrive
/// as values, so `IF` can ignore an error in the branch it does not take.
pub trait Function {
    /// The uppercase name used in formulas.
    fn name(&self) -> &'static str;
    /// One line shown during completion, with the arguments it takes.
    fn help(&self) -> &'static str;
    fn call(&self, args: &[Arg]) -> CellValue;
}

/// Converts a value to a number the way arithmetic does.
pub fn to_number(value: &CellValue) -> Result<f64, CellError> {
    match value {
        CellValue::Number(n) => Ok(*n),
        CellValue::Bool(b) => Ok(f64::from(u8::from(*b))),
        CellValue::Empty => Ok(0.0),
        CellValue::Text(text) => text.trim().parse().ok().filter(|n: &f64| n.is_finite()).ok_or(CellError::Value),
        CellValue::Error(error) => Err(*error),
    }
}

/// Converts a value to a boolean the way conditions do.
pub fn to_bool(value: &CellValue) -> Result<bool, CellError> {
    match value {
        CellValue::Bool(b) => Ok(*b),
        CellValue::Number(n) => Ok(*n != 0.0),
        CellValue::Empty => Ok(false),
        CellValue::Text(text) if text.eq_ignore_ascii_case("TRUE") => Ok(true),
        CellValue::Text(text) if text.eq_ignore_ascii_case("FALSE") => Ok(false),
        CellValue::Text(_) => Err(CellError::Value),
        CellValue::Error(error) => Err(*error),
    }
}
