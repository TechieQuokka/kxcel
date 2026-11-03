//! `AVERAGEIFS`, `MAXIFS`, and `MINIFS`: a range of numbers, and pairs of
//! range and condition picking which of them count, as in `SUMIFS`.

use excel_model::{CellError, CellValue, Doc};

use super::cells;
use super::sumifs::pairs;
use crate::{Arg, Function};

/// The numbers in the first range at positions meeting every condition. An
/// error among them is returned.
fn matching_numbers(args: &[Arg]) -> Result<Vec<f64>, CellError> {
    let (target, rest) = args.split_first().ok_or(CellError::Value)?;
    let target = cells(target)?;
    let pairs = pairs(rest, (target.rows, target.cols))?;
    let mut numbers = Vec::new();
    for (row, col, value) in target.entries() {
        if !pairs.iter().all(|(cells, condition)| condition.matches(cells.get(*row, *col))) {
            continue;
        }
        match value {
            CellValue::Number(n) => numbers.push(*n),
            CellValue::Error(error) => return Err(*error),
            _ => {}
        }
    }
    Ok(numbers)
}

pub(crate) struct AverageIfs;

impl Function for AverageIfs {
    fn name(&self) -> &'static str {
        "AVERAGEIFS"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Average of numbers meeting several conditions",
            usage: "AVERAGEIFS(average_range, range1, condition1, ...)",
            description: "Averages the numbers in average_range where every range meets its condition at the same position. All ranges must be the same size. Conditions work as in COUNTIF. With no numbers, the result is #DIV/0!.",
            args: &[
                ("average_range", "Cells to average"),
                ("range1", "Cells to check"),
                ("condition1", "What range1 must meet, such as \">10\" or \"사과\""),
                ("...", "Optional. More pairs of range and condition"),
            ],
            examples: &[("=AVERAGEIFS(C1:C100, A1:A100, \"사과\", B1:B100, \">10\")", "")],
            related: &["AVERAGEIF", "SUMIFS", "COUNTIFS"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match matching_numbers(args) {
            Ok(numbers) if numbers.is_empty() => CellValue::Error(CellError::Div0),
            Ok(numbers) => CellValue::Number(numbers.iter().sum::<f64>() / numbers.len() as f64),
            Err(error) => CellValue::Error(error),
        }
    }
}

pub(crate) struct MaxIfs;

impl Function for MaxIfs {
    fn name(&self) -> &'static str {
        "MAXIFS"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Largest number meeting conditions",
            usage: "MAXIFS(max_range, range1, condition1, ...)",
            description: "The largest number in max_range where every range meets its condition at the same position. All ranges must be the same size. Conditions work as in COUNTIF. With no numbers, the result is 0.",
            args: &[
                ("max_range", "Cells to look through"),
                ("range1", "Cells to check"),
                ("condition1", "What range1 must meet, such as \">10\" or \"사과\""),
                ("...", "Optional. More pairs of range and condition"),
            ],
            examples: &[("=MAXIFS(C1:C100, A1:A100, \"사과\")", "")],
            related: &["MINIFS", "MAX", "SUMIFS"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match matching_numbers(args) {
            Ok(numbers) => CellValue::Number(numbers.into_iter().reduce(f64::max).unwrap_or(0.0)),
            Err(error) => CellValue::Error(error),
        }
    }
}

pub(crate) struct MinIfs;

impl Function for MinIfs {
    fn name(&self) -> &'static str {
        "MINIFS"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Smallest number meeting conditions",
            usage: "MINIFS(min_range, range1, condition1, ...)",
            description: "The smallest number in min_range where every range meets its condition at the same position. All ranges must be the same size. Conditions work as in COUNTIF. With no numbers, the result is 0.",
            args: &[
                ("min_range", "Cells to look through"),
                ("range1", "Cells to check"),
                ("condition1", "What range1 must meet, such as \">10\" or \"사과\""),
                ("...", "Optional. More pairs of range and condition"),
            ],
            examples: &[("=MINIFS(C1:C100, A1:A100, \"사과\")", "")],
            related: &["MAXIFS", "MIN", "SUMIFS"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match matching_numbers(args) {
            Ok(numbers) => CellValue::Number(numbers.into_iter().reduce(f64::min).unwrap_or(0.0)),
            Err(error) => CellValue::Error(error),
        }
    }
}
