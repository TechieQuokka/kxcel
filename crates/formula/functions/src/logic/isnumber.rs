use excel_model::{CellError, CellValue, Doc};

use super::raw;
use crate::{Arg, Function};

pub(crate) struct IsNumber;

impl Function for IsNumber {
    fn name(&self) -> &'static str {
        "ISNUMBER"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Logic",
            summary: "Whether a value is a number",
            usage: "ISNUMBER(value)",
            description: "TRUE for numbers, dates included. Text that looks like a number is still text.",
            args: &[("value", "A cell or value")],
            examples: &[("=ISNUMBER(5)", "TRUE"), ("=ISNUMBER(\"5\")", "FALSE")],
            related: &["ISTEXT", "ISBLANK"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [value] = args else {
            return CellValue::Error(CellError::Value);
        };
        match raw(value) {
            Ok(value) => CellValue::Bool(matches!(value, CellValue::Number(_))),
            Err(error) => CellValue::Error(error),
        }
    }
}
