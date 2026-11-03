use excel_model::{CellError, CellValue, Doc};

use crate::{Arg, Function, scalar_number};

pub(crate) struct Days;

impl Function for Days {
    fn name(&self) -> &'static str {
        "DAYS"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Date",
            summary: "Days between two dates",
            usage: "DAYS(end, start)",
            description: "How many days from start to end, negative when end comes first. The same as end - start, written so it reads as a count of days. Note that end comes first.",
            args: &[("end", "The later date"), ("start", "The earlier date")],
            examples: &[
                ("=DAYS(DATE(2026, 12, 25), DATE(2026, 11, 2))", "53"),
                ("=DAYS(DATE(2026, 1, 1), DATE(2026, 1, 2))", "-1"),
            ],
            related: &["DATEDIF", "NETWORKDAYS", "TODAY"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [end, start] = args else {
            return CellValue::Error(CellError::Value);
        };
        let result = (|| Ok(scalar_number(end)?.floor() - scalar_number(start)?.floor()))();
        result.map_or_else(CellValue::Error, CellValue::Number)
    }
}
