use excel_model::{CellValue, Doc};

use super::numbers;
use crate::{Arg, Function};

pub(crate) struct Min;

impl Function for Min {
    fn name(&self) -> &'static str {
        "MIN"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Smallest number",
            usage: "MIN(values...)",
            description: "The smallest of the numbers given. With no numbers, the result is 0.",
            args: &[("values", "Numbers, cells, or ranges, separated by commas")],
            examples: &[("=MIN(3, 9, 4)", "3"), ("=MIN(A1:A10)", "")],
            related: &["MAX"],
        }
    }

    /// The smallest number, or 0 when there are none, as in Excel.
    fn call(&self, args: &[Arg]) -> CellValue {
        match numbers(args) {
            Ok(values) => CellValue::Number(values.into_iter().reduce(f64::min).unwrap_or(0.0)),
            Err(error) => CellValue::Error(error),
        }
    }
}
