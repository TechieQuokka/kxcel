use excel_model::{CellError, CellValue, Doc};

use super::{Mode, line, position, search, shown};
use crate::math::cells;
use crate::{Arg, Function, scalar, scalar_number, to_bool};

pub(crate) struct HLookup;

impl Function for HLookup {
    fn name(&self) -> &'static str {
        "HLOOKUP"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Lookup",
            summary: "Find a value in the first row of a table",
            usage: "HLOOKUP(value, table, row_number, [nearest])",
            description: "Looks for value in the first row of table and gives the value in the same column from row row_number of the table, counting from 1. With nearest FALSE, only an exact match counts, text ignores case, and * and ? are wildcards. With nearest TRUE, the default as in Excel, the first row must be sorted ascending and the largest value not above value is used. Not found gives #N/A.",
            args: &[
                ("value", "What to look for"),
                ("table", "The cells to search, with the values to look for in its first row"),
                ("row_number", "Which row of the table to give a value from, from 1"),
                ("nearest", "Optional. FALSE for an exact match. TRUE by default"),
            ],
            examples: &[("=HLOOKUP(\"사과\", A2:C20, 3, FALSE)", ""), ("=HLOOKUP(85, A2:B6, 2)", "")],
            related: &["XLOOKUP", "VLOOKUP", "INDEX", "MATCH"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let (wanted, table, index, nearest) = match args {
            [wanted, table, index] => (wanted, table, index, None),
            [wanted, table, index, nearest] => (wanted, table, index, Some(nearest)),
            _ => return CellValue::Error(CellError::Value),
        };
        let result = (|| {
            let table = cells(table)?;
            let nearest = match nearest {
                Some(nearest) => to_bool(&scalar(nearest)?)?,
                None => true,
            };
            let index = position(scalar_number(index)?, table.rows)?;
            let mode = if nearest { Mode::NotAbove } else { Mode::Exact };
            let found = search(&scalar(wanted)?, &line(table, 0, true), mode)?;
            Ok(shown(table.get(index, found)))
        })();
        result.unwrap_or_else(CellValue::Error)
    }
}
