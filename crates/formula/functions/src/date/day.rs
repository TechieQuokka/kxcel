use excel_model::{CellError, CellValue, Doc};

use super::parts;
use crate::{Arg, Function};

pub(crate) struct Day;

impl Function for Day {
    fn name(&self) -> &'static str {
        "DAY"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Date",
            summary: "Day of the month of a date",
            usage: "DAY(date)",
            description: "The day of the month of a date, as a number.",
            args: &[("date", "A date")],
            examples: &[("=DAY(DATE(2026, 11, 2))", "2")],
            related: &["DATE", "YEAR", "MONTH", "DAY"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [date] = args else {
            return CellValue::Error(CellError::Value);
        };
        parts(date).map_or_else(CellValue::Error, |(_, _, day)| CellValue::Number(day as f64))
    }
}
