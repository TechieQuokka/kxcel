pub(crate) mod and;
pub(crate) mod if_fn;
pub(crate) mod iferror;
pub(crate) mod ifna;
pub(crate) mod ifs;
pub(crate) mod isblank;
pub(crate) mod iserror;
pub(crate) mod isnumber;
pub(crate) mod istext;
pub(crate) mod not;
pub(crate) mod or;

use excel_model::{CellError, CellValue};

use crate::{Arg, to_bool};

/// The value of a single cell or typed value, with an empty cell kept empty,
/// unlike [`crate::scalar`]. A range of several cells is `#VALUE!`.
pub(crate) fn raw(arg: &Arg) -> Result<CellValue, CellError> {
    match arg {
        Arg::Value(value) => Ok(value.clone()),
        Arg::Cells(cells) if cells.size() == 1 => Ok(cells.get(0, 0).clone()),
        Arg::Cells(_) => Err(CellError::Value),
    }
}

/// The truth values `AND` and `OR` combine. Typed values must read as true or
/// false. In cells, numbers and booleans count and text and blanks are
/// skipped, as in Excel. Any error is returned, and so is `#VALUE!` when
/// nothing counts.
pub(crate) fn conditions(args: &[Arg]) -> Result<Vec<bool>, CellError> {
    let mut found = Vec::new();
    for arg in args {
        match arg {
            Arg::Value(value) => found.push(to_bool(value)?),
            Arg::Cells(cells) => {
                for value in cells.values() {
                    match value {
                        CellValue::Bool(b) => found.push(*b),
                        CellValue::Number(n) => found.push(*n != 0.0),
                        CellValue::Error(error) => return Err(*error),
                        _ => {}
                    }
                }
            }
        }
    }
    if found.is_empty() { Err(CellError::Value) } else { Ok(found) }
}
