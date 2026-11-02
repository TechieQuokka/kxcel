use excel_model::{CellError, CellValue, Doc};

use crate::{Arg, Function, scalar, to_bool};

pub(crate) struct If;

impl Function for If {
    fn name(&self) -> &'static str {
        "IF"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Logic",
            summary: "Pick a value by a condition",
            usage: "IF(condition, then, [else])",
            description: "Gives then when the condition is true, otherwise else. Only the chosen branch \
                          matters, so an error in the other one is ignored. Without else, a false condition \
                          gives FALSE.",
            args: &[
                ("condition", "A comparison or value, such as A1>10"),
                ("then", "The result when the condition is true"),
                ("else", "Optional. The result when it is false"),
            ],
            examples: &[("=IF(5>3, \"yes\", \"no\")", "yes"), ("=IF(B2=0, 0, A2/B2)", "")],
            ..Doc::BLANK
        }
    }

    /// `IF(condition, then, [else])`. Without an else, a false condition gives FALSE.
    fn call(&self, args: &[Arg]) -> CellValue {
        if !(2..=3).contains(&args.len()) {
            return CellValue::Error(CellError::Value);
        }
        let condition = match scalar(&args[0]) {
            Ok(value) => to_bool(&value),
            Err(error) => Err(error),
        };
        match condition {
            Ok(true) => scalar(&args[1]).unwrap_or_else(CellValue::Error),
            Ok(false) => match args.get(2) {
                Some(arg) => scalar(arg).unwrap_or_else(CellValue::Error),
                None => CellValue::Bool(false),
            },
            Err(error) => CellValue::Error(error),
        }
    }
}
