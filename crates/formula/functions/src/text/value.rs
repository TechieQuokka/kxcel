use excel_model::{CellError, CellValue, Doc};

use super::format::parse_number;
use crate::{Arg, Function, scalar};

pub(crate) struct Value;

impl Function for Value {
    fn name(&self) -> &'static str {
        "VALUE"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Text to a number",
            usage: "VALUE(text)",
            description: "Reads a number from text. Commas between thousands, a % sign, and dates such as 2026-11-02 are understood. Anything else gives #VALUE!.",
            args: &[("text", "Text that shows a number")],
            examples: &[("=VALUE(\"1,234.5\")", "1234.5"), ("=VALUE(\"15%\")", "0.15")],
            related: &["TEXT"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [text] = args else {
            return CellValue::Error(CellError::Value);
        };
        match scalar(text) {
            Ok(CellValue::Number(n)) => CellValue::Number(n),
            Ok(CellValue::Text(text)) => {
                parse_number(&text).map_or(CellValue::Error(CellError::Value), CellValue::Number)
            }
            Ok(CellValue::Error(error)) | Err(error) => CellValue::Error(error),
            Ok(_) => CellValue::Error(CellError::Value),
        }
    }
}
