use excel_model::{CellError, CellValue, Doc, MAX_DATE_SERIAL};

use super::{holidays, is_weekday, parts};
use crate::{Arg, Function, scalar_number};

pub(crate) struct Workday;

impl Function for Workday {
    fn name(&self) -> &'static str {
        "WORKDAY"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Date",
            summary: "The date some working days later",
            usage: "WORKDAY(start, days, [holidays])",
            description: "The date a number of working days, Monday to Friday, after start, skipping any dates in holidays, such as a delivery date. Negative days count back. Start itself is not counted.",
            args: &[
                ("start", "The starting date"),
                ("days", "How many working days later, or earlier when negative"),
                ("holidays", "Optional. Cells with dates that are not working days"),
            ],
            examples: &[
                ("=WORKDAY(DATE(2026, 11, 6), 1)", "2026-11-09"),
                ("=WORKDAY(DATE(2026, 11, 2), 10)", "2026-11-16"),
                ("=WORKDAY(DATE(2026, 11, 2), 1, DATE(2026, 11, 3))", "2026-11-04"),
            ],
            related: &["NETWORKDAYS", "WEEKDAY", "EDATE"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let result = (|| {
            let (start, days, off) = match args {
                [start, days] => (start, days, None),
                [start, days, off] => (start, days, Some(off)),
                _ => return Err(CellError::Value),
            };
            parts(start)?;
            let off = holidays(off)?;
            let days = scalar_number(days)?.trunc();
            if days.abs() > MAX_DATE_SERIAL {
                return Err(CellError::Num);
            }
            let step = if days < 0.0 { -1.0 } else { 1.0 };
            let (mut date, mut left) = (scalar_number(start)?.floor(), days.abs() as u64);
            while left > 0 {
                date += step;
                if !(1.0..=MAX_DATE_SERIAL).contains(&date) {
                    return Err(CellError::Num);
                }
                if is_weekday(date) && !off.contains(&(date as i64)) {
                    left -= 1;
                }
            }
            Ok(date)
        })();
        result.map_or_else(CellValue::Error, CellValue::Number)
    }

    fn returns_date(&self) -> bool {
        true
    }
}
