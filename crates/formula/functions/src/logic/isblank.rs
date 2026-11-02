use excel_model::{CellError, CellValue, Doc};

use super::raw;
use crate::{Arg, Function};

pub(crate) struct IsBlank;

impl Function for IsBlank {
    fn name(&self) -> &'static str {
        "ISBLANK"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Logic",
            summary: "Whether a cell is empty",
            usage: "ISBLANK(value)",
            description: "TRUE only for an empty cell. A cell holding empty text is not blank.",
            args: &[("value", "A cell or value")],
            examples: &[("=ISBLANK(A1)", "")],
            related: &["ISNUMBER", "ISTEXT"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [value] = args else {
            return CellValue::Error(CellError::Value);
        };
        match raw(value) {
            Ok(value) => CellValue::Bool(matches!(value, CellValue::Empty)),
            Err(error) => CellValue::Error(error),
        }
    }
}
