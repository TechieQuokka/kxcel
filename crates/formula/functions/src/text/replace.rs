use excel_model::{CellError, CellValue, Doc};

use super::{count_arg, text_arg, text_result};
use crate::{Arg, Function};

pub(crate) struct Replace;

impl Function for Replace {
    fn name(&self) -> &'static str {
        "REPLACE"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Replace characters by position",
            usage: "REPLACE(text, start, count, new)",
            description: "Replaces count characters of text, from position start, with new. A start past the end adds new at the end. SUBSTITUTE replaces by matching text instead.",
            args: &[
                ("text", "The text to change"),
                ("start", "Where to start, from 1"),
                ("count", "How many characters to replace, 0 to insert"),
                ("new", "The text to put in their place"),
            ],
            examples: &[
                ("=REPLACE(\"010-1234-5678\", 5, 4, \"****\")", "010-****-5678"),
                ("=REPLACE(\"2026년\", 1, 2, \"\")", "26년"),
            ],
            related: &["SUBSTITUTE", "MID", "SEARCH"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [text, start, count, new] = args else {
            return CellValue::Error(CellError::Value);
        };
        text_result((|| {
            let (text, start, count, new) =
                (text_arg(text)?, count_arg(start, 1.0)?, count_arg(count, 0.0)?, text_arg(new)?);
            let chars: Vec<char> = text.chars().collect();
            let from = (start - 1).min(chars.len());
            let to = from.saturating_add(count).min(chars.len());
            let mut result: String = chars[..from].iter().collect();
            result.push_str(&new);
            result.extend(&chars[to..]);
            Ok(result)
        })())
    }
}
