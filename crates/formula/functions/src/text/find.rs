use excel_model::{CellError, CellValue, Doc};

use super::{count_arg, text_arg};
use crate::{Arg, Function};

pub(crate) struct Find;

impl Function for Find {
    fn name(&self) -> &'static str {
        "FIND"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Position of text inside text",
            usage: "FIND(find, within, [start])",
            description: "The position, from 1, where find first appears in within, at or after start. Capitals matter. Not found gives #VALUE!.",
            args: &[
                ("find", "The text to look for"),
                ("within", "The text to look in"),
                ("start", "Optional. Where to start, from 1"),
            ],
            examples: &[("=FIND(\"과\", \"사과나무\")", "2"), ("=FIND(\"-\", \"2026-11-02\", 6)", "8")],
            related: &["MID", "SUBSTITUTE"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let (find, within, start) = match args {
            [find, within] => (find, within, Ok(1)),
            [find, within, start] => (find, within, count_arg(start, 1.0)),
            _ => return CellValue::Error(CellError::Value),
        };
        let result = (|| {
            let (find, within, start) = (text_arg(find)?, text_arg(within)?, start?);
            let chars: Vec<char> = within.chars().collect();
            if start > chars.len() + 1 {
                return Err(CellError::Value);
            }
            let rest: String = chars[start - 1..].iter().collect();
            let index = rest.find(&find).ok_or(CellError::Value)?;
            Ok((start + rest[..index].chars().count()) as f64)
        })();
        result.map_or_else(CellValue::Error, CellValue::Number)
    }
}
