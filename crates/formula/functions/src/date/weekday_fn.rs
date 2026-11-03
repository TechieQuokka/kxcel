use excel_model::{CellError, CellValue, Doc};

use super::{parts, weekday};
use crate::{Arg, Function, scalar_number};

pub(crate) struct Weekday;

impl Function for Weekday {
    fn name(&self) -> &'static str {
        "WEEKDAY"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Date",
            summary: "Day of the week as a number",
            usage: "WEEKDAY(date, [type])",
            description: "The day of the week of a date. Type 1, the default, counts Sunday as 1 to Saturday as 7. Type 2 counts Monday as 1 to Sunday as 7, and type 3 Monday as 0 to Sunday as 6. Other types are #NUM!.",
            args: &[("date", "A date"), ("type", "Optional. 1, 2, or 3, how days are numbered")],
            examples: &[
                ("=WEEKDAY(DATE(2026, 11, 2))", "2"),
                ("=WEEKDAY(DATE(2026, 11, 2), 2)", "1"),
                ("=WEEKDAY(DATE(2026, 11, 8), 2)", "7"),
            ],
            related: &["NETWORKDAYS", "WORKDAY", "DAY"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let result = (|| {
            let (date, kind) = match args {
                [date] => (date, 1.0),
                [date, kind] => (date, scalar_number(kind)?.trunc()),
                _ => return Err(CellError::Value),
            };
            parts(date)?;
            let sunday_first = weekday(scalar_number(date)?);
            let monday_first = (sunday_first + 6) % 7;
            match kind as i64 {
                1 => Ok(sunday_first + 1),
                2 => Ok(monday_first + 1),
                3 => Ok(monday_first),
                _ => Err(CellError::Num),
            }
        })();
        result.map_or_else(CellValue::Error, |day| CellValue::Number(f64::from(day)))
    }
}
