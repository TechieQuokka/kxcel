use excel_model::{CellError, CellValue, Doc};

use crate::{Arg, Function, scalar, to_bool};

pub(crate) struct Ifs;

impl Function for Ifs {
    fn name(&self) -> &'static str {
        "IFS"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Logic",
            summary: "The value of the first true condition",
            usage: "IFS(condition1, value1, ...)",
            description: "Checks conditions in order and gives the value paired with the first true one. With none true, the result is #N/A. Use TRUE as the last condition for a default.",
            args: &[
                ("condition1", "A comparison or value"),
                ("value1", "The result when condition1 is true"),
                ("...", "Optional. More pairs of condition and value"),
            ],
            examples: &[("=IFS(A1>=90, \"A\", A1>=80, \"B\", TRUE, \"C\")", "")],
            related: &["IF", "IFERROR"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        if args.is_empty() || !args.len().is_multiple_of(2) {
            return CellValue::Error(CellError::Value);
        }
        for pair in args.chunks(2) {
            match scalar(&pair[0]).and_then(|value| to_bool(&value)) {
                Ok(true) => return scalar(&pair[1]).unwrap_or_else(CellValue::Error),
                Ok(false) => {}
                Err(error) => return CellValue::Error(error),
            }
        }
        CellValue::Error(CellError::Na)
    }
}
