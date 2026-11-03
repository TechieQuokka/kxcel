//! `TEXTBEFORE` and `TEXTAFTER`: the text on either side of a delimiter.

use excel_model::{CellError, CellValue, Doc};

use super::{text_arg, text_result};
use crate::{Arg, Function, scalar_number};

/// The byte range of the chosen delimiter in `text`: the nth from the start,
/// or from the end when `instance` is negative. Capitals matter.
fn delimiter_at(text: &str, delimiter: &str, instance: i64) -> Result<(usize, usize), CellError> {
    if instance == 0 {
        return Err(CellError::Value);
    }
    if delimiter.is_empty() {
        let at = if instance > 0 { 0 } else { text.len() };
        return Ok((at, at));
    }
    let found: Vec<usize> = text.match_indices(delimiter).map(|(at, _)| at).collect();
    let index =
        if instance > 0 { instance as usize - 1 } else { found.len().wrapping_sub(instance.unsigned_abs() as usize) };
    found.get(index).map(|at| (*at, at + delimiter.len())).ok_or(CellError::Na)
}

fn split(args: &[Arg], before: bool) -> CellValue {
    let (text, delimiter, instance) = match args {
        [text, delimiter] => (text, delimiter, Ok(1.0)),
        [text, delimiter, instance] => (text, delimiter, scalar_number(instance)),
        _ => return CellValue::Error(CellError::Value),
    };
    text_result((|| {
        let (text, delimiter, instance) = (text_arg(text)?, text_arg(delimiter)?, instance?.trunc());
        if instance.abs() > text.len() as f64 + 1.0 {
            return Err(CellError::Na);
        }
        let (start, end) = delimiter_at(&text, &delimiter, instance as i64)?;
        Ok(if before { text[..start].to_owned() } else { text[end..].to_owned() })
    })())
}

const ARGS: &[(&str, &str)] = &[
    ("text", "The text to look in"),
    ("delimiter", "The text that marks where to cut, capitals mattering"),
    ("instance", "Optional. Which delimiter, 1 for the first, -1 for the last"),
];

pub(crate) struct TextBefore;

impl Function for TextBefore {
    fn name(&self) -> &'static str {
        "TEXTBEFORE"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Text before a delimiter",
            usage: "TEXTBEFORE(text, delimiter, [instance])",
            description: "The text before the first delimiter, or before the instance-th one. A negative instance counts from the end. Not found is #N/A, which IFNA can catch.",
            args: ARGS,
            examples: &[
                ("=TEXTBEFORE(\"kim@example.com\", \"@\")", "kim"),
                ("=TEXTBEFORE(\"서울-강남-역삼\", \"-\", 2)", "서울-강남"),
                ("=TEXTBEFORE(\"서울-강남-역삼\", \"-\", -1)", "서울-강남"),
            ],
            related: &["TEXTAFTER", "LEFT", "FIND"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        split(args, true)
    }
}

pub(crate) struct TextAfter;

impl Function for TextAfter {
    fn name(&self) -> &'static str {
        "TEXTAFTER"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Text after a delimiter",
            usage: "TEXTAFTER(text, delimiter, [instance])",
            description: "The text after the first delimiter, or after the instance-th one. A negative instance counts from the end. Not found is #N/A, which IFNA can catch.",
            args: ARGS,
            examples: &[
                ("=TEXTAFTER(\"kim@example.com\", \"@\")", "example.com"),
                ("=TEXTAFTER(\"서울-강남-역삼\", \"-\", -1)", "역삼"),
            ],
            related: &["TEXTBEFORE", "RIGHT", "MID"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        split(args, false)
    }
}
