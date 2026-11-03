use excel_model::{CellError, CellValue, Doc};

use crate::{Arg, Function, scalar, scalar_number};

pub(crate) struct Choose;

impl Function for Choose {
    fn name(&self) -> &'static str {
        "CHOOSE"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Lookup",
            summary: "Pick a value by its number",
            usage: "CHOOSE(index, value1, value2, ...)",
            description: "Gives value1 when index is 1, value2 when it is 2, and so on. index is cut to a whole number, and outside 1 to the count of values it is #VALUE!.",
            args: &[
                ("index", "Which value, from 1"),
                ("value1", "The value for 1"),
                ("...", "Optional. The values for 2, 3, and so on"),
            ],
            examples: &[
                ("=CHOOSE(2, \"금\", \"은\", \"동\")", "은"),
                ("=CHOOSE(WEEKDAY(DATE(2026, 11, 2)), \"일\", \"월\", \"화\", \"수\", \"목\", \"금\", \"토\")", "월"),
            ],
            related: &["SWITCH", "INDEX", "WEEKDAY"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let Some((index, values)) = args.split_first() else {
            return CellValue::Error(CellError::Value);
        };
        let result = (|| {
            let index = scalar_number(index)?.trunc();
            if index < 1.0 || index > values.len() as f64 {
                return Err(CellError::Value);
            }
            scalar(&values[index as usize - 1])
        })();
        result.unwrap_or_else(CellValue::Error)
    }
}
