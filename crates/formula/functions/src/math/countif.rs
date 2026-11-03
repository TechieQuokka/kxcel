use excel_model::{CellError, CellValue, Doc};

use super::cells;
use crate::{Arg, Function, scalar};
use excel_model::Criterion;

pub(crate) struct CountIf;

impl Function for CountIf {
    fn name(&self) -> &'static str {
        "COUNTIF"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Count cells meeting a condition",
            usage: "COUNTIF(range, condition)",
            description: "Counts the cells in range that meet the condition. A condition is a value, or text with a comparison such as \">5\" or \"<>apple\". In text, * stands for any characters and ? for one.",
            args: &[("range", "Cells to check"), ("condition", "What to count, such as 5, \">=10\", or \"a*\"")],
            examples: &[
                ("=COUNTIF(A1:A10, \">5\")", ""),
                ("=COUNTIF(B1:B100, \"사과\")", ""),
                ("=COUNTIF(C1:C9, \"\")", ""),
            ],
            related: &["COUNTIFS", "SUMIF", "AVERAGEIF"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [range, condition] = args else {
            return CellValue::Error(CellError::Value);
        };
        let (cells, condition) = match (cells(range), scalar(condition)) {
            (Ok(cells), Ok(condition)) => (cells, Criterion::new(&condition)),
            (Err(error), _) | (_, Err(error)) => return CellValue::Error(error),
        };
        let mut count = cells.values().filter(|value| condition.matches(value)).count() as u64;
        if condition.matches(&CellValue::Empty) {
            count += cells.size() - cells.values().count() as u64;
        }
        CellValue::Number(count as f64)
    }
}
