use excel_model::{CellError, CellValue, Doc};

use super::finite;
use crate::{Arg, Function, scalar_number};

pub(crate) struct Int;

impl Function for Int {
    fn name(&self) -> &'static str {
        "INT"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Round down to a whole number",
            usage: "INT(number)",
            description: "Rounds down to the nearest whole number, so negative numbers move away from zero.",
            args: &[("number", "A number")],
            examples: &[("=INT(7.9)", "7"), ("=INT(-7.1)", "-8")],
            related: &["ROUNDDOWN", "MOD"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [number] = args else {
            return CellValue::Error(CellError::Value);
        };
        scalar_number(number).map_or_else(CellValue::Error, |n| finite(n.floor()))
    }
}
