use excel_model::{CellValue, Doc};

use super::conditions;
use crate::{Arg, Function};

pub(crate) struct And;

impl Function for And {
    fn name(&self) -> &'static str {
        "AND"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Logic",
            summary: "True when every condition is true",
            usage: "AND(conditions...)",
            description: "In ranges, numbers and TRUE or FALSE count, and text and empty cells are skipped. Nonzero numbers are true.",
            args: &[("conditions", "Comparisons, values, or ranges, separated by commas")],
            examples: &[("=AND(1<2, 3>2)", "TRUE"), ("=AND(A1>0, A1<10)", "")],
            related: &["OR", "NOT", "IF"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match conditions(args) {
            Ok(found) => CellValue::Bool(found.iter().all(|b| *b)),
            Err(error) => CellValue::Error(error),
        }
    }
}
