use excel_model::{CellValue, Doc};

use crate::{Arg, Function};

pub(crate) struct CountA;

impl Function for CountA {
    fn name(&self) -> &'static str {
        "COUNTA"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Count values that are not empty",
            usage: "COUNTA(values...)",
            description: "Counts cells holding anything, text and errors included, and every typed value.",
            args: &[("values", "Values, cells, or ranges, separated by commas")],
            examples: &[("=COUNTA(1, \"a\", TRUE)", "3"), ("=COUNTA(A1:A100)", "")],
            related: &["COUNT", "COUNTBLANK"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let count: usize = args
            .iter()
            .map(|arg| match arg {
                Arg::Value(_) => 1,
                Arg::Cells(cells) => cells.values().count(),
            })
            .sum();
        CellValue::Number(count as f64)
    }
}
