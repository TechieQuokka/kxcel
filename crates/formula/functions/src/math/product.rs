use excel_model::{CellValue, Doc};

use super::{finite, numbers};
use crate::{Arg, Function};

pub(crate) struct Product;

impl Function for Product {
    fn name(&self) -> &'static str {
        "PRODUCT"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Multiply numbers",
            usage: "PRODUCT(values...)",
            description: "Multiplies every number given. In ranges, text and empty cells are skipped. With no numbers, the result is 0.",
            args: &[("values", "Numbers, cells, or ranges, separated by commas")],
            examples: &[("=PRODUCT(2, 3, 4)", "24"), ("=PRODUCT(A1:A5)", "")],
            related: &["SUM"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match numbers(args) {
            Ok(values) if values.is_empty() => CellValue::Number(0.0),
            Ok(values) => finite(values.iter().product()),
            Err(error) => CellValue::Error(error),
        }
    }
}
