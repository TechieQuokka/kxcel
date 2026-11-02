use excel_model::CellValue;

use super::numbers;
use crate::{Arg, Function};

pub(crate) struct Min;

impl Function for Min {
    fn name(&self) -> &'static str {
        "MIN"
    }

    fn help(&self) -> &'static str {
        "MIN(values...)  Smallest number"
    }

    /// The smallest number, or 0 when there are none, as in Excel.
    fn call(&self, args: &[Arg]) -> CellValue {
        match numbers(args) {
            Ok(values) => CellValue::Number(values.into_iter().reduce(f64::min).unwrap_or(0.0)),
            Err(error) => CellValue::Error(error),
        }
    }
}
