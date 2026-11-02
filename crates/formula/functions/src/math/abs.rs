use excel_model::{CellError, CellValue, Doc};

use super::finite;
use crate::{Arg, Function, scalar_number};

pub(crate) struct Abs;

impl Function for Abs {
    fn name(&self) -> &'static str {
        "ABS"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Absolute value",
            usage: "ABS(number)",
            description: "The number without its sign.",
            args: &[("number", "A number")],
            examples: &[("=ABS(-4.5)", "4.5")],
            related: &["INT"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [number] = args else {
            return CellValue::Error(CellError::Value);
        };
        scalar_number(number).map_or_else(CellValue::Error, |n| finite(n.abs()))
    }
}
