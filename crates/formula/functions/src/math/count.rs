use excel_model::{CellValue, Doc};

use crate::{Arg, Function, to_number};

pub(crate) struct Count;

impl Function for Count {
    fn name(&self) -> &'static str {
        "COUNT"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Count numbers",
            usage: "COUNT(values...)",
            description: "Counts the values that are numbers. Text and empty cells are not counted.",
            args: &[("values", "Values, cells, or ranges, separated by commas")],
            examples: &[("=COUNT(1, \"a\", 3)", "2"), ("=COUNT(A1:A100)", "")],
            related: &["SUM", "AVERAGE"],
        }
    }

    /// Counts numbers. Unlike other functions, errors are skipped, not returned.
    fn call(&self, args: &[Arg]) -> CellValue {
        let count: usize = args
            .iter()
            .map(|arg| match arg {
                Arg::Value(value) => usize::from(to_number(value).is_ok()),
                Arg::Cells(cells) => cells.values().filter(|value| matches!(value, CellValue::Number(_))).count(),
            })
            .sum();
        CellValue::Number(count as f64)
    }
}
