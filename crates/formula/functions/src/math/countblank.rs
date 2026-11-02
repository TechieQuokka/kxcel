use excel_model::{CellError, CellValue, Doc};

use super::cells;
use crate::{Arg, Function};

pub(crate) struct CountBlank;

impl Function for CountBlank {
    fn name(&self) -> &'static str {
        "COUNTBLANK"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Count empty cells",
            usage: "COUNTBLANK(range)",
            description: "Counts empty cells, and cells holding empty text.",
            args: &[("range", "Cells to look at")],
            examples: &[("=COUNTBLANK(A1:A10)", "")],
            related: &["COUNTA", "COUNTIF"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [range] = args else {
            return CellValue::Error(CellError::Value);
        };
        let cells = match cells(range) {
            Ok(cells) => cells,
            Err(error) => return CellValue::Error(error),
        };
        let empty_text =
            cells.values().filter(|value| matches!(value, CellValue::Text(text) if text.is_empty())).count();
        let filled = cells.values().count();
        CellValue::Number((cells.size() - filled as u64 + empty_text as u64) as f64)
    }
}
