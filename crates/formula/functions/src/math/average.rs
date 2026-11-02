use excel_model::{CellError, CellValue};

use super::numbers;
use crate::{Arg, Function};

pub(crate) struct Average;

impl Function for Average {
    fn name(&self) -> &'static str {
        "AVERAGE"
    }

    fn help(&self) -> &'static str {
        "AVERAGE(values...)  Average of numbers"
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match numbers(args) {
            Ok(values) if values.is_empty() => CellValue::Error(CellError::Div0),
            Ok(values) => CellValue::Number(values.iter().sum::<f64>() / values.len() as f64),
            Err(error) => CellValue::Error(error),
        }
    }
}
