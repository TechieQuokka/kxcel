use excel_model::{CellError, CellValue, Doc};

use super::finite;
use crate::{Arg, Function, scalar_number};

pub(crate) struct Sqrt;

impl Function for Sqrt {
    fn name(&self) -> &'static str {
        "SQRT"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Square root",
            usage: "SQRT(number)",
            description: "The positive square root. A negative number gives #NUM!.",
            args: &[("number", "A number zero or above")],
            examples: &[("=SQRT(16)", "4")],
            related: &["POWER"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [number] = args else {
            return CellValue::Error(CellError::Value);
        };
        match scalar_number(number) {
            Ok(n) if n < 0.0 => CellValue::Error(CellError::Num),
            Ok(n) => finite(n.sqrt()),
            Err(error) => CellValue::Error(error),
        }
    }
}
