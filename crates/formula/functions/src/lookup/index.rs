use excel_model::{CellError, CellValue, Doc};

use super::{position, shown};
use crate::math::cells;
use crate::{Arg, Function, scalar_number};

pub(crate) struct Index;

impl Function for Index {
    fn name(&self) -> &'static str {
        "INDEX"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Lookup",
            summary: "The value at a position in a range",
            usage: "INDEX(range, row, [column])",
            description: "Gives the value at row and column of range, counting from 1. For a single row or column, one number is enough. A position outside the range gives #REF!.",
            args: &[
                ("range", "The cells to pick from"),
                ("row", "The row, from 1"),
                ("column", "Optional. The column, from 1"),
            ],
            examples: &[("=INDEX(B2:B20, 3)", ""), ("=INDEX(A1:C10, MATCH(\"사과\", A1:A10, 0), 3)", "")],
            related: &["MATCH", "XLOOKUP"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let result = (|| {
            let (range, row, col) = match args {
                [range, row] => (cells(range)?, scalar_number(row)?, None),
                [range, row, col] => (cells(range)?, scalar_number(row)?, Some(scalar_number(col)?)),
                _ => return Err(CellError::Value),
            };
            let (row, col) = match col {
                Some(col) => (position(row, range.rows)?, position(col, range.cols)?),
                None if range.cols == 1 => (position(row, range.rows)?, 0),
                None if range.rows == 1 => (0, position(row, range.cols)?),
                None => return Err(CellError::Ref),
            };
            Ok(shown(range.get(row, col)))
        })();
        result.unwrap_or_else(CellValue::Error)
    }
}
