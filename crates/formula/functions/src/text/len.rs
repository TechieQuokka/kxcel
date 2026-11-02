use excel_model::{CellError, CellValue, Doc};

use super::text_arg;
use crate::{Arg, Function};

pub(crate) struct Len;

impl Function for Len {
    fn name(&self) -> &'static str {
        "LEN"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Number of characters",
            usage: "LEN(text)",
            description: "Counts characters, so 사과 is 2. Numbers count as they are shown.",
            args: &[("text", "Text or a value")],
            examples: &[("=LEN(\"사과 주스\")", "5"), ("=LEN(12.5)", "4")],
            related: &["LEFT", "MID", "TRIM"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [text] = args else {
            return CellValue::Error(CellError::Value);
        };
        text_arg(text).map_or_else(CellValue::Error, |text| CellValue::Number(text.chars().count() as f64))
    }
}
