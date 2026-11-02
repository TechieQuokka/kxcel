use excel_model::{CellError, CellValue, Doc};

use super::{text_arg, text_result, to_text};
use crate::{Arg, Function, scalar, to_bool};

pub(crate) struct TextJoin;

impl Function for TextJoin {
    fn name(&self) -> &'static str {
        "TEXTJOIN"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Join text with a separator",
            usage: "TEXTJOIN(separator, skip_empty, values...)",
            description: "Joins values with separator between them, ranges in reading order. With skip_empty TRUE, empty cells and empty text are left out.",
            args: &[
                ("separator", "Text put between values, such as \", \""),
                ("skip_empty", "TRUE to leave out empty values"),
                ("values", "Text, values, cells, or ranges"),
            ],
            examples: &[("=TEXTJOIN(\", \", TRUE, A1:A5)", ""), ("=TEXTJOIN(\"-\", FALSE, 2026, 11, 2)", "2026-11-2")],
            related: &["CONCAT"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [separator, skip_empty, values @ ..] = args else {
            return CellValue::Error(CellError::Value);
        };
        if values.is_empty() {
            return CellValue::Error(CellError::Value);
        }
        let result = (|| {
            let separator = text_arg(separator)?;
            let skip_empty = to_bool(&scalar(skip_empty)?)?;
            let mut parts = Vec::new();
            for arg in values {
                match arg {
                    Arg::Value(value) => parts.push(to_text(value)?),
                    // Empty cells are left out of the entries, so they are
                    // added back by position when they count.
                    Arg::Cells(cells) if !skip_empty => {
                        for row in 0..cells.rows {
                            for col in 0..cells.cols {
                                parts.push(to_text(cells.get(row, col))?);
                            }
                        }
                    }
                    Arg::Cells(cells) => {
                        for value in cells.values() {
                            parts.push(to_text(value)?);
                        }
                    }
                }
            }
            if skip_empty {
                parts.retain(|part| !part.is_empty());
            }
            Ok(parts.join(&separator))
        })();
        text_result(result)
    }
}
