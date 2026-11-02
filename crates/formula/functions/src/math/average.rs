use excel_model::{CellError, CellValue, Doc};

use super::numbers;
use crate::{Arg, Function};

pub(crate) struct Average;

impl Function for Average {
    fn name(&self) -> &'static str {
        "AVERAGE"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Average of numbers",
            usage: "AVERAGE(values...)",
            description: "The sum of the numbers divided by how many there are. In ranges, text and empty \
                          cells are skipped. With no numbers, the result is #DIV/0!.",
            args: &[("values", "Numbers, cells, or ranges, separated by commas")],
            examples: &[("=AVERAGE(2, 4, 9)", "5"), ("=AVERAGE(B2:B20)", "")],
            related: &["SUM", "COUNT"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match numbers(args) {
            Ok(values) if values.is_empty() => CellValue::Error(CellError::Div0),
            Ok(values) => CellValue::Number(values.iter().sum::<f64>() / values.len() as f64),
            Err(error) => CellValue::Error(error),
        }
    }
}
