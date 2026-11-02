use excel_model::CellValue;

use super::numbers;
use crate::{Arg, Function};

pub(crate) struct Sum;

impl Function for Sum {
    fn name(&self) -> &'static str {
        "SUM"
    }

    fn help(&self) -> &'static str {
        "SUM(values...)  Add numbers"
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match numbers(args) {
            Ok(values) => CellValue::Number(values.iter().sum()),
            Err(error) => CellValue::Error(error),
        }
    }
}
