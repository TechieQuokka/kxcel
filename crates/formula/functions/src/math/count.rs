use excel_model::CellValue;

use crate::{Arg, Function, to_number};

pub(crate) struct Count;

impl Function for Count {
    fn name(&self) -> &'static str {
        "COUNT"
    }

    fn help(&self) -> &'static str {
        "COUNT(values...)  Count numbers"
    }

    /// Counts numbers. Unlike other functions, errors are skipped, not returned.
    fn call(&self, args: &[Arg]) -> CellValue {
        let count: usize = args
            .iter()
            .map(|arg| match arg {
                Arg::Value(value) => usize::from(to_number(value).is_ok()),
                Arg::Cells(values) => values.iter().filter(|value| matches!(value, CellValue::Number(_))).count(),
            })
            .sum();
        CellValue::Number(count as f64)
    }
}
