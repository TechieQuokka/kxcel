use excel_model::CellValue;

use super::numbers;
use crate::{Arg, Function};

pub(crate) struct Max;

impl Function for Max {
    fn name(&self) -> &'static str {
        "MAX"
    }

    fn help(&self) -> &'static str {
        "MAX(values...)  Largest number"
    }

    /// The largest number, or 0 when there are none, as in Excel.
    fn call(&self, args: &[Arg]) -> CellValue {
        match numbers(args) {
            Ok(values) => CellValue::Number(values.into_iter().reduce(f64::max).unwrap_or(0.0)),
            Err(error) => CellValue::Error(error),
        }
    }
}
