use excel_model::{CellError, CellValue, Doc};

use super::{count_arg, text_arg, text_result};
use crate::{Arg, Function};

pub(crate) struct Right;

impl Function for Right {
    fn name(&self) -> &'static str {
        "RIGHT"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Characters from the end",
            usage: "RIGHT(text, [count])",
            description: "Takes characters from the end of text.",
            args: &[("text", "Text or a value"), ("count", "Optional. How many characters. 1 by default")],
            examples: &[("=RIGHT(\"2026-11-02\", 2)", "02")],
            related: &["LEFT", "MID"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let (text, count) = match args {
            [text] => (text_arg(text), Ok(1)),
            [text, count] => (text_arg(text), count_arg(count, 0.0)),
            _ => return CellValue::Error(CellError::Value),
        };
        text_result(text.and_then(|text| {
            count.map(|count| {
                let chars: Vec<char> = text.chars().collect();
                chars[chars.len().saturating_sub(count)..].iter().collect()
            })
        }))
    }
}
