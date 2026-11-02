use excel_model::{CellError, CellValue, Doc};

use super::sumifs::pairs;
use crate::criteria::matching_positions;
use crate::{Arg, Function};

pub(crate) struct CountIfs;

impl Function for CountIfs {
    fn name(&self) -> &'static str {
        "COUNTIFS"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Count cells meeting several conditions",
            usage: "COUNTIFS(range1, condition1, ...)",
            description: "Counts positions where every range meets its condition. All ranges must be the same size. Conditions work as in COUNTIF.",
            args: &[
                ("range1", "Cells to check"),
                ("condition1", "What range1 must meet"),
                ("...", "Optional. More pairs of range and condition"),
            ],
            examples: &[("=COUNTIFS(A1:A100, \"사과\", B1:B100, \">10\")", "")],
            related: &["COUNTIF", "SUMIFS"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let Some(Arg::Cells(first)) = args.first() else {
            return CellValue::Error(CellError::Value);
        };
        let shape = (first.rows, first.cols);
        match pairs(args, shape) {
            Ok(pairs) => CellValue::Number(matching_positions(shape, &pairs).len() as f64),
            Err(error) => CellValue::Error(error),
        }
    }
}
