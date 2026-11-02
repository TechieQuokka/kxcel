use excel_model::{CellError, CellValue, Doc};

use crate::{Arg, Function, scalar};

pub(crate) struct IfError;

impl Function for IfError {
    fn name(&self) -> &'static str {
        "IFERROR"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Logic",
            summary: "A fallback when a value is an error",
            usage: "IFERROR(value, fallback)",
            description: "Gives value, unless it is an error such as #DIV/0! or #N/A, then gives fallback.",
            args: &[("value", "What to try"), ("fallback", "The result when value is an error")],
            examples: &[("=IFERROR(1/0, 0)", "0"), ("=IFERROR(A2/B2, \"\")", "")],
            related: &["ISERROR", "IF"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [value, fallback] = args else {
            return CellValue::Error(CellError::Value);
        };
        match scalar(value) {
            Ok(CellValue::Error(_)) | Err(_) => scalar(fallback).unwrap_or_else(CellValue::Error),
            Ok(value) => value,
        }
    }
}
