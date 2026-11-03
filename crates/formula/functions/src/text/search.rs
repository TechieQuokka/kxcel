use excel_model::{CellError, CellValue, Doc};

use super::{count_arg, text_arg};
use crate::{Arg, Function};

pub(crate) struct Search;

impl Function for Search {
    fn name(&self) -> &'static str {
        "SEARCH"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Position of text inside text, ignoring case",
            usage: "SEARCH(find, within, [start])",
            description: "The position, from 1, where find first appears in within, at or after start, as FIND gives, but ignoring capitals. In find, * stands for any run of characters, ? for one character, and ~ before either makes it plain. Not found gives #VALUE!.",
            args: &[
                ("find", "The text to look for"),
                ("within", "The text to look in"),
                ("start", "Optional. Where to start, from 1"),
            ],
            examples: &[
                ("=SEARCH(\"apple\", \"Big Apple\")", "5"),
                ("=SEARCH(\"a?c\", \"xxABC\")", "3"),
                ("=ISNUMBER(SEARCH(\"강남\", A2))", ""),
            ],
            related: &["FIND", "MID", "REPLACE"],
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
            let chars: Vec<char> = within.to_lowercase().chars().collect();
            if start > chars.len() + 1 {
                return Err(CellError::Value);
            }
            let pattern = parse(&find.to_lowercase());
            (start - 1..=chars.len())
                .find(|from| matches_prefix(&pattern, &chars[*from..]))
                .map(|from| (from + 1) as f64)
                .ok_or(CellError::Value)
        })();
        result.map_or_else(CellValue::Error, CellValue::Number)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Char(char),
    One,
    Any,
}

fn parse(pattern: &str) -> Vec<Part> {
    let mut parts = Vec::new();
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        parts.push(match c {
            '~' => Part::Char(chars.next().unwrap_or('~')),
            '?' => Part::One,
            '*' => Part::Any,
            c => Part::Char(c),
        });
    }
    parts
}

/// Whether the pattern matches the start of `text`, any amount of it.
fn matches_prefix(parts: &[Part], text: &[char]) -> bool {
    // matched[j]: whether the parts so far match the first j characters.
    let mut matched = vec![false; text.len() + 1];
    matched[0] = true;
    for part in parts {
        let mut next = vec![false; text.len() + 1];
        for j in 0..=text.len() {
            next[j] = match part {
                Part::Any => matched[j] || (j > 0 && next[j - 1]),
                Part::One => j > 0 && matched[j - 1],
                Part::Char(c) => j > 0 && matched[j - 1] && text[j - 1] == *c,
            };
        }
        matched = next;
    }
    matched.contains(&true)
}
