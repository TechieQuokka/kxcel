pub(crate) mod average;
pub(crate) mod count;
pub(crate) mod max;
pub(crate) mod min;
pub(crate) mod sum;

use excel_model::{CellError, CellValue};

use crate::{Arg, to_number};

/// Collects the numbers a numeric function works on, as Excel does.
///
/// Values read from cells count only if they are numbers. Typed values are
/// converted, so `SUM("2", TRUE)` is 3, and text that is not a number is an
/// error. Any error among the arguments is returned.
pub(crate) fn numbers(args: &[Arg]) -> Result<Vec<f64>, CellError> {
    let mut found = Vec::new();
    for arg in args {
        match arg {
            Arg::Value(value) => found.push(to_number(value)?),
            Arg::Cells(values) => {
                for value in values {
                    match value {
                        CellValue::Number(n) => found.push(*n),
                        CellValue::Error(error) => return Err(*error),
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(found)
}
