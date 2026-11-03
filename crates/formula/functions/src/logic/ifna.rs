use excel_model::{CellError, CellValue, Doc};

use crate::{Arg, Function, scalar};

pub(crate) struct IfNa;

impl Function for IfNa {
    fn name(&self) -> &'static str {
        "IFNA"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Logic",
            summary: "A fallback when a value is #N/A",
            usage: "IFNA(value, fallback)",
            description: "Gives value, unless it is #N/A, as from a lookup that found nothing, then gives fallback. Other errors stay, so mistakes are not hidden as IFERROR would hide them.",
            args: &[("value", "What to try, such as a lookup"), ("fallback", "The result when value is #N/A")],
            examples: &[
                ("=IFNA(XLOOKUP(A2, D2:D100, E2:E100), \"없음\")", ""),
                ("=IFNA(VLOOKUP(A2, D2:E9, 2, FALSE), 0)", ""),
            ],
            related: &["IFERROR", "XLOOKUP", "VLOOKUP"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [value, fallback] = args else {
            return CellValue::Error(CellError::Value);
        };
        match scalar(value) {
            Ok(CellValue::Error(CellError::Na)) => scalar(fallback).unwrap_or_else(CellValue::Error),
            Ok(value) => value,
            Err(error) => CellValue::Error(error),
        }
    }
}
