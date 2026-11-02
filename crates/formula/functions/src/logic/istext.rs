use excel_model::{CellError, CellValue, Doc};

use super::raw;
use crate::{Arg, Function};

pub(crate) struct IsText;

impl Function for IsText {
    fn name(&self) -> &'static str {
        "ISTEXT"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Logic",
            summary: "Whether a value is text",
            usage: "ISTEXT(value)",
            description: "TRUE for text, including empty text.",
            args: &[("value", "A cell or value")],
            examples: &[("=ISTEXT(\"사과\")", "TRUE"), ("=ISTEXT(5)", "FALSE")],
            related: &["ISNUMBER", "ISBLANK"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [value] = args else {
            return CellValue::Error(CellError::Value);
        };
        match raw(value) {
            Ok(value) => CellValue::Bool(matches!(value, CellValue::Text(_))),
            Err(error) => CellValue::Error(error),
        }
    }
}
