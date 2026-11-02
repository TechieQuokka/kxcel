use excel_model::{CellError, CellValue, Doc};

use super::parts;
use crate::{Arg, Function};

pub(crate) struct Year;

impl Function for Year {
    fn name(&self) -> &'static str {
        "YEAR"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Date",
            summary: "Year of a date",
            usage: "YEAR(date)",
            description: "The year of a date, as a number.",
            args: &[("date", "A date")],
            examples: &[("=YEAR(DATE(2026, 11, 2))", "2026")],
            related: &["DATE", "YEAR", "MONTH", "DAY"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [date] = args else {
            return CellValue::Error(CellError::Value);
        };
        parts(date).map_or_else(CellValue::Error, |(year, _, _)| CellValue::Number(year as f64))
    }
}
