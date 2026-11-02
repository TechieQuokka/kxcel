use excel_model::{CellError, CellValue, Doc};

use crate::{Arg, Function, scalar, to_bool};

pub(crate) struct Not;

impl Function for Not {
    fn name(&self) -> &'static str {
        "NOT"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Logic",
            summary: "The opposite truth value",
            usage: "NOT(condition)",
            description: "TRUE becomes FALSE and FALSE becomes TRUE.",
            args: &[("condition", "A comparison or value")],
            examples: &[("=NOT(1>2)", "TRUE")],
            related: &["AND", "OR"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [condition] = args else {
            return CellValue::Error(CellError::Value);
        };
        match scalar(condition).and_then(|value| to_bool(&value)) {
            Ok(b) => CellValue::Bool(!b),
            Err(error) => CellValue::Error(error),
        }
    }
}
