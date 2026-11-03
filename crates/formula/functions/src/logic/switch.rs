use excel_model::{CellError, CellValue, Doc};

use crate::{Arg, Function, scalar};

pub(crate) struct Switch;

impl Function for Switch {
    fn name(&self) -> &'static str {
        "SWITCH"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Logic",
            summary: "Pick a result by matching a value",
            usage: "SWITCH(value, match1, result1, ..., [default])",
            description: "Compares value with each match in turn and gives the result after the first one equal to it. Text compares ignoring case. A last argument without a pair is the default; with no match and no default, the result is #N/A. Easier to read than IF inside IF.",
            args: &[
                ("value", "What to compare"),
                ("match1", "A value to compare with"),
                ("result1", "The result when value equals match1"),
                ("...", "Optional. More pairs, then a default"),
            ],
            examples: &[
                ("=SWITCH(\"B\", \"A\", 0.1, \"B\", 0.05, 0)", "0.05"),
                ("=SWITCH(3, 1, \"one\", 2, \"two\", \"many\")", "many"),
                ("=SWITCH(A2, \"VIP\", 0.2, \"회원\", 0.1, 0)", ""),
            ],
            related: &["IFS", "IF", "CHOOSE"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let Some((value, rest)) = args.split_first() else {
            return CellValue::Error(CellError::Value);
        };
        if rest.len() < 2 {
            return CellValue::Error(CellError::Value);
        }
        let result = (|| {
            let value = scalar(value)?;
            if let CellValue::Error(error) = value {
                return Err(error);
            }
            for pair in rest.chunks(2) {
                match pair {
                    [candidate, result] => {
                        if crate::lookup::equals(&scalar(candidate)?, &value, false) {
                            return scalar(result);
                        }
                    }
                    [default] => return scalar(default),
                    _ => unreachable!("chunks of two"),
                }
            }
            Err(CellError::Na)
        })();
        result.unwrap_or_else(CellValue::Error)
    }
}
