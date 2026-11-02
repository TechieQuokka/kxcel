use excel_model::{CellError, CellValue, Doc};

use super::{Rounding, round};
use crate::{Arg, Function, scalar_number};

pub(crate) struct Round;

impl Function for Round {
    fn name(&self) -> &'static str {
        "ROUND"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Round to a number of digits",
            usage: "ROUND(number, digits)",
            description: "Rounds half away from zero. Negative digits round to tens, hundreds, and so on.",
            args: &[("number", "A number"), ("digits", "Decimal places to keep. Negative for tens and above")],
            examples: &[("=ROUND(2.675, 2)", "2.68"), ("=ROUND(-1.5, 0)", "-2"), ("=ROUND(1234, -2)", "1200")],
            related: &["ROUNDUP", "ROUNDDOWN", "INT"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [number, digits] = args else {
            return CellValue::Error(CellError::Value);
        };
        match (scalar_number(number), scalar_number(digits)) {
            (Ok(n), Ok(d)) => round(n, d, Rounding::Nearest),
            (Err(error), _) | (_, Err(error)) => CellValue::Error(error),
        }
    }
}
