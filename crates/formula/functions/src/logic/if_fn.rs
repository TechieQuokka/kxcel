use excel_model::{CellError, CellValue};

use crate::{Arg, Function, to_bool};

pub(crate) struct If;

impl Function for If {
    fn name(&self) -> &'static str {
        "IF"
    }

    fn help(&self) -> &'static str {
        "IF(condition, then, [else])  Pick by a condition"
    }

    /// `IF(condition, then, [else])`. Without an else, a false condition gives FALSE.
    fn call(&self, args: &[Arg]) -> CellValue {
        if !(2..=3).contains(&args.len()) {
            return CellValue::Error(CellError::Value);
        }
        let condition = match single(&args[0]) {
            Ok(value) => to_bool(&value),
            Err(error) => Err(error),
        };
        match condition {
            Ok(true) => single(&args[1]).unwrap_or_else(CellValue::Error),
            Ok(false) => match args.get(2) {
                Some(arg) => single(arg).unwrap_or_else(CellValue::Error),
                None => CellValue::Bool(false),
            },
            Err(error) => CellValue::Error(error),
        }
    }
}

/// Reads one value from an argument. A range of several cells is not one value.
fn single(arg: &Arg) -> Result<CellValue, CellError> {
    match arg {
        Arg::Value(value) => Ok(value.clone()),
        Arg::Cells(values) => match values.as_slice() {
            [] => Ok(CellValue::Number(0.0)),
            [value] => Ok(value.clone()),
            _ => Err(CellError::Value),
        },
    }
}
