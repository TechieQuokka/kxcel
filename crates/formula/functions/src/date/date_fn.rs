use excel_model::{CellError, CellValue, Doc};

use super::normalized;
use crate::{Arg, Function, scalar_number};

pub(crate) struct Date;

impl Function for Date {
    fn name(&self) -> &'static str {
        "DATE"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Date",
            summary: "A date from year, month, and day",
            usage: "DATE(year, month, day)",
            description: "Builds a date. Months and days past their end carry over, so month 13 is January of the next year and day 0 is the last day of the month before. Years below 1900 count from 1900.",
            args: &[
                ("year", "The year, such as 2026"),
                ("month", "The month, 1 to 12, or beyond to carry over"),
                ("day", "The day of the month, or beyond to carry over"),
            ],
            examples: &[
                ("=DATE(2026, 11, 2)", "2026-11-02"),
                ("=DATE(2026, 13, 1)", "2027-01-01"),
                ("=DATE(2026, 3, 0)", "2026-02-28"),
            ],
            related: &["YEAR", "MONTH", "DAY", "TODAY"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [year, month, day] = args else {
            return CellValue::Error(CellError::Value);
        };
        let result = (|| normalized(scalar_number(year)?, scalar_number(month)?, scalar_number(day)?))();
        result.map_or_else(CellValue::Error, CellValue::Number)
    }

    fn returns_date(&self) -> bool {
        true
    }
}
