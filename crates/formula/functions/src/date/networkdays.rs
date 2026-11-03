use excel_model::{CellError, CellValue, Doc};

use super::{holidays, is_weekday, parts};
use crate::{Arg, Function, scalar_number};

pub(crate) struct NetworkDays;

impl Function for NetworkDays {
    fn name(&self) -> &'static str {
        "NETWORKDAYS"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Date",
            summary: "Working days between two dates",
            usage: "NETWORKDAYS(start, end, [holidays])",
            description: "Counts Monday to Friday from start to end, both included, leaving out any dates in holidays. When end is before start, the count is negative.",
            args: &[
                ("start", "The first date"),
                ("end", "The last date"),
                ("holidays", "Optional. Cells with dates that are not working days"),
            ],
            examples: &[
                ("=NETWORKDAYS(DATE(2026, 11, 2), DATE(2026, 11, 13))", "10"),
                ("=NETWORKDAYS(DATE(2026, 11, 2), DATE(2026, 11, 13), DATE(2026, 11, 3))", "9"),
                ("=NETWORKDAYS(DATE(2026, 1, 1), DATE(2026, 12, 31), H2:H20)", ""),
            ],
            related: &["WORKDAY", "WEEKDAY", "DATEDIF"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let result = (|| {
            let (start, end, off) = match args {
                [start, end] => (start, end, None),
                [start, end, off] => (start, end, Some(off)),
                _ => return Err(CellError::Value),
            };
            parts(start)?;
            parts(end)?;
            let off = holidays(off)?;
            let (start, end) = (scalar_number(start)?.floor(), scalar_number(end)?.floor());
            let (first, last, sign) = if start <= end { (start, end, 1.0) } else { (end, start, -1.0) };
            let days = last - first + 1.0;
            // Whole weeks have five working days, and the days left over are counted one by one.
            let weeks = (days / 7.0).floor();
            let rest = (0..(days - weeks * 7.0) as i64)
                .filter(|offset| is_weekday(first + weeks * 7.0 + *offset as f64))
                .count() as f64;
            let off = off.iter().filter(|day| (first..=last).contains(&(**day as f64)) && is_weekday(**day as f64));
            Ok(sign * (weeks * 5.0 + rest - off.count() as f64))
        })();
        result.map_or_else(CellValue::Error, CellValue::Number)
    }
}
