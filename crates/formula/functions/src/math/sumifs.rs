use excel_model::{CellError, CellValue, Doc};

use super::{cells, sum_and_count};
use crate::criteria::Criterion;
use crate::{Arg, Cells, Function, scalar};

pub(crate) struct SumIfs;

impl Function for SumIfs {
    fn name(&self) -> &'static str {
        "SUMIFS"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Add numbers meeting several conditions",
            usage: "SUMIFS(sum_range, range1, condition1, ...)",
            description: "Adds the numbers in sum_range where every range meets its condition at the same position. All ranges must be the same size. Conditions work as in COUNTIF.",
            args: &[
                ("sum_range", "Cells to add"),
                ("range1", "Cells to check"),
                ("condition1", "What range1 must meet"),
                ("...", "Optional. More pairs of range and condition"),
            ],
            examples: &[("=SUMIFS(C1:C100, A1:A100, \"사과\", B1:B100, \">10\")", "")],
            related: &["SUMIF", "COUNTIFS"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let Some((summed, rest)) = args.split_first() else {
            return CellValue::Error(CellError::Value);
        };
        let result = cells(summed).and_then(|summed| {
            let pairs = pairs(rest, (summed.rows, summed.cols))?;
            let values = summed.entries().iter().filter(|(row, col, _)| {
                pairs.iter().all(|(cells, condition)| condition.matches(cells.get(*row, *col)))
            });
            sum_and_count(values.map(|(_, _, value)| value))
        });
        match result {
            Ok((total, _)) => CellValue::Number(total),
            Err(error) => CellValue::Error(error),
        }
    }
}

/// Reads pairs of range and condition, each range the size `shape`.
pub(crate) fn pairs(args: &[Arg], shape: (u32, u32)) -> Result<Vec<(&Cells, Criterion)>, CellError> {
    if args.is_empty() || !args.len().is_multiple_of(2) {
        return Err(CellError::Value);
    }
    args.chunks(2)
        .map(|pair| {
            let range = cells(&pair[0])?;
            if (range.rows, range.cols) != shape {
                return Err(CellError::Value);
            }
            Ok((range, Criterion::new(&scalar(&pair[1])?)))
        })
        .collect()
}
