use excel_model::{CellError, CellValue, Doc};

use super::{Rounding, round};
use crate::{Arg, Function, scalar_number};

pub(crate) struct RoundDown;

impl Function for RoundDown {
    fn name(&self) -> &'static str {
        "ROUNDDOWN"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Round toward zero",
            usage: "ROUNDDOWN(number, digits)",
            description: "Cuts off digits, rounding toward zero.",
            args: &[("number", "A number"), ("digits", "Decimal places to keep. Negative for tens and above")],
            examples: &[("=ROUNDDOWN(3.99, 1)", "3.9"), ("=ROUNDDOWN(-3.99, 0)", "-3")],
            related: &["ROUND", "ROUNDUP", "INT"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [number, digits] = args else {
            return CellValue::Error(CellError::Value);
        };
        match (scalar_number(number), scalar_number(digits)) {
            (Ok(n), Ok(d)) => round(n, d, Rounding::Down),
            (Err(error), _) | (_, Err(error)) => CellValue::Error(error),
        }
    }
}
