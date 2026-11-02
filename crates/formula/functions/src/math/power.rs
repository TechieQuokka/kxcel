use excel_model::{CellError, CellValue, Doc};

use super::finite;
use crate::{Arg, Function, scalar_number};

pub(crate) struct Power;

impl Function for Power {
    fn name(&self) -> &'static str {
        "POWER"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "A number raised to a power",
            usage: "POWER(number, power)",
            description: "The same as number^power.",
            args: &[("number", "The base"), ("power", "The exponent")],
            examples: &[("=POWER(2, 10)", "1024"), ("=POWER(9, 0.5)", "3")],
            related: &["SQRT"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [number, power] = args else {
            return CellValue::Error(CellError::Value);
        };
        match (scalar_number(number), scalar_number(power)) {
            (Ok(base), Ok(exponent)) => finite(base.powf(exponent)),
            (Err(error), _) | (_, Err(error)) => CellValue::Error(error),
        }
    }
}
