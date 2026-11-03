use excel_model::{CellError, CellValue, Doc};

use super::{cells, sum_and_count};
use crate::{Arg, Function, scalar};
use excel_model::Criterion;

pub(crate) struct SumIf;

impl Function for SumIf {
    fn name(&self) -> &'static str {
        "SUMIF"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Add numbers meeting a condition",
            usage: "SUMIF(range, condition, [sum_range])",
            description: "Adds the numbers in sum_range where the matching cell of range meets the condition. Without sum_range, adds range itself. Conditions work as in COUNTIF.",
            args: &[
                ("range", "Cells to check"),
                ("condition", "Which cells count, such as \">5\" or \"apple\""),
                ("sum_range", "Optional. Cells to add, lined up with range"),
            ],
            examples: &[("=SUMIF(A1:A10, \">0\")", ""), ("=SUMIF(A1:A100, \"사과\", B1:B100)", "")],
            related: &["SUMIFS", "COUNTIF", "AVERAGEIF"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match matching_sum(args) {
            Ok((total, _)) => CellValue::Number(total),
            Err(error) => CellValue::Error(error),
        }
    }
}

/// The sum and count of numbers that `SUMIF` and `AVERAGEIF` work on. Values
/// are taken from the summed cells lined up with each checked cell, so only
/// positions with a value to add are looked at.
pub(crate) fn matching_sum(args: &[Arg]) -> Result<(f64, usize), CellError> {
    let (range, condition, summed) = match args {
        [range, condition] => (cells(range)?, scalar(condition)?, cells(range)?),
        [range, condition, summed] => (cells(range)?, scalar(condition)?, cells(summed)?),
        _ => return Err(CellError::Value),
    };
    let condition = Criterion::new(&condition);
    let values = summed
        .entries()
        .iter()
        .filter(|(row, col, _)| *row < range.rows && *col < range.cols && condition.matches(range.get(*row, *col)))
        .map(|(_, _, value)| value);
    sum_and_count(values)
}
