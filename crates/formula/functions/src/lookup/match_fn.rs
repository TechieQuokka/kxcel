use excel_model::{CellError, CellValue, Doc};

use super::{Mode, line, search};
use crate::math::cells;
use crate::{Arg, Function, scalar, scalar_number};

pub(crate) struct Match;

impl Function for Match {
    fn name(&self) -> &'static str {
        "MATCH"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Lookup",
            summary: "Position of a value in a row or column",
            usage: "MATCH(value, range, [type])",
            description: "Gives where value is in range, counting from 1. Type 0 finds an exact match, with * and ? as wildcards in text. Type 1, the default as in Excel, finds the largest value not above value in data sorted ascending, and -1 the smallest not below in data sorted descending. Not found gives #N/A.",
            args: &[
                ("value", "What to look for"),
                ("range", "A single row or column"),
                ("type", "Optional. 0 exact, 1 nearest below, -1 nearest above. 1 by default"),
            ],
            examples: &[("=MATCH(\"배\", A1:A10, 0)", ""), ("=MATCH(85, B1:B5)", "")],
            related: &["INDEX", "XLOOKUP"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let result = (|| {
            let (wanted, range, kind) = match args {
                [wanted, range] => (scalar(wanted)?, cells(range)?, 1.0),
                [wanted, range, kind] => (scalar(wanted)?, cells(range)?, scalar_number(kind)?),
                _ => return Err(CellError::Value),
            };
            let across = match (range.rows, range.cols) {
                (_, 1) => false,
                (1, _) => true,
                _ => return Err(CellError::Na),
            };
            let mode = match kind {
                k if k > 0.0 => Mode::NotAbove,
                k if k < 0.0 => Mode::NotBelow,
                _ => Mode::Exact,
            };
            let found = search(&wanted, &line(range, 0, across), mode)?;
            Ok(CellValue::Number(f64::from(found + 1)))
        })();
        result.unwrap_or_else(CellValue::Error)
    }
}
