use excel_model::{CellError, CellValue, Doc};

use super::{count_arg, text_arg, text_result};
use crate::{Arg, Function};

pub(crate) struct Substitute;

impl Function for Substitute {
    fn name(&self) -> &'static str {
        "SUBSTITUTE"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Replace text",
            usage: "SUBSTITUTE(text, old, new, [which])",
            description: "Replaces old with new everywhere, or only the which-th time it appears. Capitals matter.",
            args: &[
                ("text", "Text or a value"),
                ("old", "The text to replace"),
                ("new", "What to put instead"),
                ("which", "Optional. Which appearance to replace, from 1"),
            ],
            examples: &[
                ("=SUBSTITUTE(\"2026-11-02\", \"-\", \"/\")", "2026/11/02"),
                ("=SUBSTITUTE(\"a-b-c\", \"-\", \"+\", 2)", "a-b+c"),
            ],
            related: &["TRIM", "FIND"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let (text, old, new, which) = match args {
            [text, old, new] => (text, old, new, None),
            [text, old, new, which] => (text, old, new, Some(which)),
            _ => return CellValue::Error(CellError::Value),
        };
        let result = (|| {
            let (text, old, new) = (text_arg(text)?, text_arg(old)?, text_arg(new)?);
            if old.is_empty() {
                return Ok(text);
            }
            let Some(which) = which else {
                return Ok(text.replace(&old, &new));
            };
            let which = count_arg(which, 1.0)?;
            Ok(match text.match_indices(&old).nth(which - 1) {
                Some((at, _)) => format!("{}{new}{}", &text[..at], &text[at + old.len()..]),
                None => text,
            })
        })();
        text_result(result)
    }
}
