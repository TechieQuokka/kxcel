use excel_model::{CellError, CellValue, Doc};

use super::{count_arg, text_arg, text_result};
use crate::{Arg, Function};

pub(crate) struct Mid;

impl Function for Mid {
    fn name(&self) -> &'static str {
        "MID"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Characters from the middle",
            usage: "MID(text, start, count)",
            description: "Takes count characters starting at position start, where the first character is 1.",
            args: &[
                ("text", "Text or a value"),
                ("start", "The first character's position, from 1"),
                ("count", "How many characters"),
            ],
            examples: &[("=MID(\"가나다라\", 2, 2)", "나다")],
            related: &["LEFT", "RIGHT", "FIND"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [text, start, count] = args else {
            return CellValue::Error(CellError::Value);
        };
        let result = text_arg(text).and_then(|text| {
            let start = count_arg(start, 1.0)?;
            let count = count_arg(count, 0.0)?;
            Ok(text.chars().skip(start - 1).take(count).collect())
        });
        text_result(result)
    }
}
