use excel_model::{CellError, CellValue, Doc};

use super::{count_arg, text_arg, text_result};
use crate::{Arg, Function};

/// The longest text a cell holds in Excel.
const MAX_TEXT: usize = 32_767;

pub(crate) struct Rept;

impl Function for Rept {
    fn name(&self) -> &'static str {
        "REPT"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Repeat text",
            usage: "REPT(text, times)",
            description: "text repeated a number of times, such as stars for a rating or a simple bar. times is cut to a whole number. A result longer than 32,767 characters is #VALUE!.",
            args: &[("text", "The text to repeat"), ("times", "How many times, 0 for empty text")],
            examples: &[("=REPT(\"★\", 3)", "★★★"), ("=REPT(\"■\", B2/10)", "")],
            related: &["CONCAT", "LEN"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [text, times] = args else {
            return CellValue::Error(CellError::Value);
        };
        text_result((|| {
            let (text, times) = (text_arg(text)?, count_arg(times, 0.0)?);
            if text.chars().count().saturating_mul(times) > MAX_TEXT {
                return Err(CellError::Value);
            }
            Ok(text.repeat(times))
        })())
    }
}
