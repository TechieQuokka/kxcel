use excel_model::{CellError, CellValue, Doc};

use super::raw;
use crate::{Arg, Function};

pub(crate) struct IsError;

impl Function for IsError {
    fn name(&self) -> &'static str {
        "ISERROR"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Logic",
            summary: "Whether a value is an error",
            usage: "ISERROR(value)",
            description: "TRUE for any error, such as #DIV/0!, #N/A, or #VALUE!.",
            args: &[("value", "A cell or value")],
            examples: &[("=ISERROR(1/0)", "TRUE"), ("=ISERROR(A1)", "")],
            related: &["IFERROR"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [value] = args else {
            return CellValue::Error(CellError::Value);
        };
        match raw(value) {
            Ok(value) => CellValue::Bool(matches!(value, CellValue::Error(_))),
            Err(error) => CellValue::Error(error),
        }
    }
}
