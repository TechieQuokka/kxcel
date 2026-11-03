use excel_model::{CellError, CellValue, Doc};

use super::months_later;
use crate::{Arg, Function};

pub(crate) struct EDate;

impl Function for EDate {
    fn name(&self) -> &'static str {
        "EDATE"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Date",
            summary: "The same day some months later",
            usage: "EDATE(date, months)",
            description: "The date a number of months after date, or before it when months is negative, such as a due date. When that month is shorter, the result is its last day.",
            args: &[("date", "The starting date"), ("months", "How many months later, or earlier when negative")],
            examples: &[
                ("=EDATE(DATE(2026, 11, 2), 3)", "2027-02-02"),
                ("=EDATE(DATE(2026, 1, 31), 1)", "2026-02-28"),
                ("=EDATE(DATE(2026, 11, 2), -12)", "2025-11-02"),
            ],
            related: &["EOMONTH", "DATE", "DATEDIF"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [date, months] = args else {
            return CellValue::Error(CellError::Value);
        };
        months_later(date, months, false).map_or_else(CellValue::Error, CellValue::Number)
    }

    fn returns_date(&self) -> bool {
        true
    }
}

pub(crate) struct EoMonth;

impl Function for EoMonth {
    fn name(&self) -> &'static str {
        "EOMONTH"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Date",
            summary: "The last day of a month",
            usage: "EOMONTH(date, months)",
            description: "The last day of the month a number of months after date. 0 gives the end of date's own month, and -1 the end of the month before.",
            args: &[("date", "The starting date"), ("months", "How many months later, or earlier when negative")],
            examples: &[
                ("=EOMONTH(DATE(2026, 11, 2), 0)", "2026-11-30"),
                ("=EOMONTH(DATE(2026, 1, 15), 1)", "2026-02-28"),
                ("=EOMONTH(DATE(2026, 3, 15), -1)", "2026-02-28"),
            ],
            related: &["EDATE", "DATE", "DAY"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [date, months] = args else {
            return CellValue::Error(CellError::Value);
        };
        months_later(date, months, true).map_or_else(CellValue::Error, CellValue::Number)
    }

    fn returns_date(&self) -> bool {
        true
    }
}
