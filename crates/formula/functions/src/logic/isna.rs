use excel_model::{CellError, CellValue, Doc};

use super::raw;
use crate::{Arg, Function};

pub(crate) struct IsNa;

impl Function for IsNa {
    fn name(&self) -> &'static str {
        "ISNA"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Logic",
            summary: "Whether a value is #N/A",
            usage: "ISNA(value)",
            description: "TRUE when value is #N/A, as from a lookup that found nothing, and FALSE for anything else, other errors included.",
            args: &[("value", "A value, cell, or formula")],
            examples: &[("=ISNA(MATCH(9, A1:A3, 0))", ""), ("=ISNA(1/0)", "FALSE")],
            related: &["IFNA", "ISERROR"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [value] = args else {
            return CellValue::Error(CellError::Value);
        };
        match raw(value) {
            Ok(value) => CellValue::Bool(value == CellValue::Error(CellError::Na)),
            Err(error) => CellValue::Error(error),
        }
    }
}
