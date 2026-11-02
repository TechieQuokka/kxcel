pub(crate) mod concat;
pub(crate) mod find;
pub(crate) mod format;
pub(crate) mod left;
pub(crate) mod len;
pub(crate) mod lower;
pub(crate) mod mid;
pub(crate) mod right;
pub(crate) mod substitute;
pub(crate) mod text_fn;
pub(crate) mod textjoin;
pub(crate) mod trim;
pub(crate) mod upper;
pub(crate) mod value;

use excel_model::{CellError, CellValue, format_number};

use crate::{Arg, scalar, scalar_number};

/// A value as text, the way joining it into text shows it. Numbers use the
/// general format, and an empty cell is empty text.
pub(crate) fn to_text(value: &CellValue) -> Result<String, CellError> {
    match value {
        CellValue::Empty => Ok(String::new()),
        CellValue::Number(n) => Ok(format_number(*n)),
        CellValue::Text(text) => Ok(text.clone()),
        CellValue::Bool(true) => Ok("TRUE".to_owned()),
        CellValue::Bool(false) => Ok("FALSE".to_owned()),
        CellValue::Error(error) => Err(*error),
    }
}

/// One argument as text. An empty cell is empty text, not 0.
pub(crate) fn text_arg(arg: &Arg) -> Result<String, CellError> {
    match arg {
        Arg::Cells(cells) if cells.size() == 1 => to_text(cells.get(0, 0)),
        other => to_text(&scalar(other)?),
    }
}

/// A count of characters: a whole number, at least `min`, cut toward zero.
pub(crate) fn count_arg(arg: &Arg, min: f64) -> Result<usize, CellError> {
    let n = scalar_number(arg)?.trunc();
    if n < min { Err(CellError::Value) } else { Ok(n.min(u32::MAX as f64) as usize) }
}

/// A text result, or the error that stopped it.
pub(crate) fn text_result(result: Result<String, CellError>) -> CellValue {
    result.map_or_else(CellValue::Error, CellValue::Text)
}
