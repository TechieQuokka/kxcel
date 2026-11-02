use excel_model::{CellError, CellValue, Doc};

use super::sumif::matching_sum;
use crate::{Arg, Function};

pub(crate) struct AverageIf;

impl Function for AverageIf {
    fn name(&self) -> &'static str {
        "AVERAGEIF"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Average of numbers meeting a condition",
            usage: "AVERAGEIF(range, condition, [average_range])",
            description: "Averages the numbers in average_range where the matching cell of range meets the condition. Without average_range, averages range itself. With no numbers, the result is #DIV/0!.",
            args: &[
                ("range", "Cells to check"),
                ("condition", "Which cells count, such as \">5\" or \"apple\""),
                ("average_range", "Optional. Cells to average, lined up with range"),
            ],
            examples: &[("=AVERAGEIF(B2:B20, \">0\")", ""), ("=AVERAGEIF(A1:A100, \"사과\", B1:B100)", "")],
            related: &["SUMIF", "COUNTIF", "AVERAGE"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match matching_sum(args) {
            Ok((_, 0)) => CellValue::Error(CellError::Div0),
            Ok((total, count)) => CellValue::Number(total / count as f64),
            Err(error) => CellValue::Error(error),
        }
    }
}
