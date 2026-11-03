//! `CEILING` and `FLOOR`: rounding to a multiple, such as to the nearest 100.

use excel_model::{CellError, CellValue, Doc};

use crate::{Arg, Function, scalar_number};

/// Cuts a value to 15 significant digits, as Excel shows numbers, so binary
/// noise such as 1.1 / 0.1 being 11.000000000000002 does not round up.
fn significant(value: f64) -> f64 {
    format!("{value:.14e}").parse().unwrap_or(value)
}

/// `number` rounded to a multiple of `step`, up when `up` and down otherwise,
/// as seen from zero when both are negative, as Excel does.
fn to_multiple(args: &[Arg], up: bool) -> Result<f64, CellError> {
    let (number, step) = match args {
        [number] => (scalar_number(number)?, 1.0),
        [number, step] => (scalar_number(number)?, scalar_number(step)?),
        _ => return Err(CellError::Value),
    };
    if number > 0.0 && step < 0.0 {
        return Err(CellError::Num);
    }
    if step == 0.0 {
        return if up { Ok(0.0) } else { Err(CellError::Div0) };
    }
    let times = significant(number / step);
    let times = if up { times.ceil() } else { times.floor() };
    Ok(significant(times * step))
}

const ARGS: &[(&str, &str)] =
    &[("number", "The number to round"), ("significance", "Optional. The multiple to round to, 1 if left out")];

pub(crate) struct Ceiling;

impl Function for Ceiling {
    fn name(&self) -> &'static str {
        "CEILING"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Round up to a multiple",
            usage: "CEILING(number, [significance])",
            description: "Rounds up to the nearest multiple of significance, such as to 100 or 500 won. A negative number with a negative significance rounds away from zero. A positive number with a negative significance is #NUM!.",
            args: ARGS,
            examples: &[("=CEILING(12345, 100)", "12400"), ("=CEILING(12345, 500)", "12500"), ("=CEILING(2.1)", "3")],
            related: &["FLOOR", "ROUNDUP", "ROUND"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        to_multiple(args, true).map_or_else(CellValue::Error, super::finite)
    }
}

pub(crate) struct Floor;

impl Function for Floor {
    fn name(&self) -> &'static str {
        "FLOOR"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Round down to a multiple",
            usage: "FLOOR(number, [significance])",
            description: "Rounds down to the nearest multiple of significance, such as to 1,000 won. A negative number with a negative significance rounds toward zero. A positive number with a negative significance is #NUM!, and a significance of 0 is #DIV/0!.",
            args: ARGS,
            examples: &[("=FLOOR(12345, 1000)", "12000"), ("=FLOOR(-2.5, 2)", "-4"), ("=FLOOR(2.9)", "2")],
            related: &["CEILING", "ROUNDDOWN", "INT"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        to_multiple(args, false).map_or_else(CellValue::Error, super::finite)
    }
}
