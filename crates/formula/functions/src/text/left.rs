use excel_model::{CellError, CellValue, Doc};

use super::{count_arg, text_arg, text_result};
use crate::{Arg, Function};

pub(crate) struct Left;

impl Function for Left {
    fn name(&self) -> &'static str {
        "LEFT"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Characters from the start",
            usage: "LEFT(text, [count])",
            description: "Takes characters from the start of text.",
            args: &[("text", "Text or a value"), ("count", "Optional. How many characters. 1 by default")],
            examples: &[("=LEFT(\"사과나무\", 2)", "사과")],
            related: &["RIGHT", "MID"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let (text, count) = match args {
            [text] => (text_arg(text), Ok(1)),
            [text, count] => (text_arg(text), count_arg(count, 0.0)),
            _ => return CellValue::Error(CellError::Value),
        };
        text_result(text.and_then(|text| count.map(|count| text.chars().take(count).collect())))
    }
}
