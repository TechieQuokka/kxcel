use excel_model::{CellError, CellValue, Doc};

use super::to_text;
use crate::{Arg, Function};

pub(crate) struct Concat;

impl Function for Concat {
    fn name(&self) -> &'static str {
        "CONCAT"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Join text",
            usage: "CONCAT(values...)",
            description: "Joins values into one text, with ranges joined in reading order. The same as the & operator.",
            args: &[("values", "Text, values, cells, or ranges, separated by commas")],
            examples: &[("=CONCAT(\"사과\", \" \", 3, \"개\")", "사과 3개"), ("=CONCAT(A1:C1)", "")],
            related: &["TEXTJOIN"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let mut joined = String::new();
        for arg in args {
            let parts: Result<Vec<String>, CellError> = match arg {
                Arg::Value(value) => to_text(value).map(|text| vec![text]),
                Arg::Cells(cells) => cells.values().map(to_text).collect(),
            };
            match parts {
                Ok(parts) => joined.extend(parts),
                Err(error) => return CellValue::Error(error),
            }
        }
        CellValue::Text(joined)
    }
}
