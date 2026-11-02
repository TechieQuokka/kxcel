use excel_model::{CellError, CellValue, Doc};

use super::finite;
use crate::{Arg, Function, scalar_number};

pub(crate) struct Mod;

impl Function for Mod {
    fn name(&self) -> &'static str {
        "MOD"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Remainder after division",
            usage: "MOD(number, divisor)",
            description: "The remainder has the divisor's sign, as in Excel. A divisor of 0 gives #DIV/0!.",
            args: &[("number", "The number to divide"), ("divisor", "What to divide by")],
            examples: &[("=MOD(10, 3)", "1"), ("=MOD(-10, 3)", "2")],
            related: &["INT"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [number, divisor] = args else {
            return CellValue::Error(CellError::Value);
        };
        match (scalar_number(number), scalar_number(divisor)) {
            (Ok(_), Ok(0.0)) => CellValue::Error(CellError::Div0),
            (Ok(n), Ok(d)) => finite(n - d * (n / d).floor()),
            (Err(error), _) | (_, Err(error)) => CellValue::Error(error),
        }
    }
}
