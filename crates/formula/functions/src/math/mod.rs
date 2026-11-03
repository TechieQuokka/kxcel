pub(crate) mod abs;
pub(crate) mod average;
pub(crate) mod averageif;
pub(crate) mod count;
pub(crate) mod counta;
pub(crate) mod countblank;
pub(crate) mod countif;
pub(crate) mod countifs;
pub(crate) mod ifs_range;
pub(crate) mod int;
pub(crate) mod max;
pub(crate) mod min;
pub(crate) mod mod_fn;
pub(crate) mod multiple;
pub(crate) mod power;
pub(crate) mod product;
pub(crate) mod round;
pub(crate) mod rounddown;
pub(crate) mod roundup;
pub(crate) mod sqrt;
pub(crate) mod stats;
pub(crate) mod sum;
pub(crate) mod sumif;
pub(crate) mod sumifs;
pub(crate) mod sumproduct;

use excel_model::{CellError, CellValue};

use crate::{Arg, Cells, to_number};

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
            Arg::Cells(cells) => {
                for value in cells.values() {
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

/// A number result, or `#NUM!` when it is not finite.
pub(crate) fn finite(n: f64) -> CellValue {
    if n.is_finite() { CellValue::Number(n) } else { CellValue::Error(CellError::Num) }
}

/// How [`round`] treats the digits it drops.
#[derive(Clone, Copy)]
pub(crate) enum Rounding {
    /// Half away from zero, as `ROUND` does.
    Nearest,
    /// Away from zero.
    Up,
    /// Toward zero.
    Down,
}

/// Rounds to `digits` decimal places, or to tens, hundreds, and so on when
/// `digits` is negative. The scaled value is first cut to 15 significant
/// digits, so binary noise such as 2.675 being stored as 2.67499... does not
/// change the result, as Excel shows it.
pub(crate) fn round(value: f64, digits: f64, mode: Rounding) -> CellValue {
    let digits = digits.trunc().clamp(-308.0, 308.0) as i32;
    let scale = 10f64.powi(digits.abs());
    let scaled = if digits >= 0 { value * scale } else { value / scale };
    let scaled: f64 = format!("{scaled:.14e}").parse().unwrap_or(scaled);
    let rounded = match mode {
        Rounding::Nearest => scaled.round(),
        Rounding::Up => scaled.abs().ceil().copysign(scaled),
        Rounding::Down => scaled.trunc(),
    };
    finite(if digits >= 0 { rounded / scale } else { rounded * scale })
}

/// The cells of an argument that must be a reference, or `#VALUE!`.
pub(crate) fn cells(arg: &Arg) -> Result<&Cells, CellError> {
    match arg {
        Arg::Cells(cells) => Ok(cells),
        Arg::Value(CellValue::Error(error)) => Err(*error),
        Arg::Value(_) => Err(CellError::Value),
    }
}

/// Adds the numbers among `values`, and counts them. An error among them is returned.
pub(crate) fn sum_and_count<'a>(values: impl Iterator<Item = &'a CellValue>) -> Result<(f64, usize), CellError> {
    let mut total = 0.0;
    let mut count = 0;
    for value in values {
        match value {
            CellValue::Number(n) => {
                total += n;
                count += 1;
            }
            CellValue::Error(error) => return Err(*error),
            _ => {}
        }
    }
    Ok((total, count))
}
