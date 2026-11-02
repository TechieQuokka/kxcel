use excel_model::{CellValue, Doc};

use super::conditions;
use crate::{Arg, Function};

pub(crate) struct Or;

impl Function for Or {
    fn name(&self) -> &'static str {
        "OR"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Logic",
            summary: "True when any condition is true",
            usage: "OR(conditions...)",
            description: "In ranges, numbers and TRUE or FALSE count, and text and empty cells are skipped. Nonzero numbers are true.",
            args: &[("conditions", "Comparisons, values, or ranges, separated by commas")],
            examples: &[("=OR(1>2, 3>2)", "TRUE"), ("=OR(A1=\"사과\", A1=\"배\")", "")],
            related: &["AND", "NOT", "IF"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match conditions(args) {
            Ok(found) => CellValue::Bool(found.iter().any(|b| *b)),
            Err(error) => CellValue::Error(error),
        }
    }
}
