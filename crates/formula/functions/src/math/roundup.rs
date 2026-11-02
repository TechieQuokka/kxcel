use excel_model::{CellError, CellValue, Doc};

use super::{Rounding, round};
use crate::{Arg, Function, scalar_number};

pub(crate) struct RoundUp;

impl Function for RoundUp {
    fn name(&self) -> &'static str {
        "ROUNDUP"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Round away from zero",
            usage: "ROUNDUP(number, digits)",
            description: "Rounds away from zero to the given digits.",
            args: &[("number", "A number"), ("digits", "Decimal places to keep. Negative for tens and above")],
            examples: &[("=ROUNDUP(3.21, 1)", "3.3"), ("=ROUNDUP(-3.21, 1)", "-3.3")],
            related: &["ROUND", "ROUNDDOWN"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [number, digits] = args else {
            return CellValue::Error(CellError::Value);
        };
        match (scalar_number(number), scalar_number(digits)) {
            (Ok(n), Ok(d)) => round(n, d, Rounding::Up),
            (Err(error), _) | (_, Err(error)) => CellValue::Error(error),
        }
    }
}
