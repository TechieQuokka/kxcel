use excel_model::{CellValue, Doc};

use super::numbers;
use crate::{Arg, Function};

pub(crate) struct Max;

impl Function for Max {
    fn name(&self) -> &'static str {
        "MAX"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Largest number",
            usage: "MAX(values...)",
            description: "The largest of the numbers given. With no numbers, the result is 0.",
            args: &[("values", "Numbers, cells, or ranges, separated by commas")],
            examples: &[("=MAX(3, 9, 4)", "9"), ("=MAX(A1:A10)", "")],
            related: &["MIN"],
        }
    }

    /// The largest number, or 0 when there are none, as in Excel.
    fn call(&self, args: &[Arg]) -> CellValue {
        match numbers(args) {
            Ok(values) => CellValue::Number(values.into_iter().reduce(f64::max).unwrap_or(0.0)),
            Err(error) => CellValue::Error(error),
        }
    }
}
