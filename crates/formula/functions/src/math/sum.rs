use excel_model::{CellValue, Doc};

use super::numbers;
use crate::{Arg, Function};

pub(crate) struct Sum;

impl Function for Sum {
    fn name(&self) -> &'static str {
        "SUM"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Add numbers",
            usage: "SUM(values...)",
            description: "Adds every number given. In ranges, text and empty cells are skipped.",
            args: &[("values", "Numbers, cells, or ranges, separated by commas")],
            examples: &[("=SUM(1, 2, 3)", "6"), ("=SUM(A1:A10)", ""), ("=SUM(A1:A3, C1)", "")],
            related: &["AVERAGE", "COUNT"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match numbers(args) {
            Ok(values) => CellValue::Number(values.iter().sum()),
            Err(error) => CellValue::Error(error),
        }
    }
}
