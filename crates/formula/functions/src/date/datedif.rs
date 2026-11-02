use excel_model::{CellError, CellValue, Doc, date_serial};

use super::{days_in_month, parts};
use crate::{Arg, Function, scalar, scalar_number};

pub(crate) struct DateDif;

impl Function for DateDif {
    fn name(&self) -> &'static str {
        "DATEDIF"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Date",
            summary: "Time between two dates",
            usage: "DATEDIF(start, end, unit)",
            description: "Counts whole years, months, or days from start to end. Unit \"Y\" counts years, \"M\" months, \"D\" days, \"YM\" months left over after the years, \"MD\" days left over after the months, and \"YD\" days left over after the years. An end before the start gives #NUM!.",
            args: &[
                ("start", "The first date"),
                ("end", "The last date"),
                ("unit", "\"Y\", \"M\", \"D\", \"YM\", \"MD\", or \"YD\""),
            ],
            examples: &[
                ("=DATEDIF(DATE(2000, 5, 10), DATE(2026, 11, 2), \"Y\")", "26"),
                ("=DATEDIF(DATE(2026, 1, 31), DATE(2026, 3, 1), \"M\")", "1"),
                ("=DATEDIF(DATE(2026, 1, 1), DATE(2026, 11, 2), \"D\")", "305"),
            ],
            related: &["DATE", "YEAR"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [start, end, unit] = args else {
            return CellValue::Error(CellError::Value);
        };
        let result = (|| {
            let (first, last) = (scalar_number(start)?.floor(), scalar_number(end)?.floor());
            let (from, to) = (parts(start)?, parts(end)?);
            if last < first {
                return Err(CellError::Num);
            }
            let unit = match scalar(unit)? {
                CellValue::Text(unit) => unit.to_uppercase(),
                _ => return Err(CellError::Value),
            };
            between(first, last, from, to, &unit)
        })();
        result.map_or_else(CellValue::Error, CellValue::Number)
    }
}

type Parts = (i64, u32, u32);

/// The count for one unit between two dates, given as serial numbers and parts.
fn between(first: f64, last: f64, from: Parts, to: Parts, unit: &str) -> Result<f64, CellError> {
    // Whole months: a month is complete once the end reaches the start's day.
    let months = (to.0 - from.0) * 12 + i64::from(to.1) - i64::from(from.1) - i64::from(to.2 < from.2);
    Ok(match unit {
        "Y" => months.div_euclid(12) as f64,
        "M" => months as f64,
        "D" => last - first,
        "YM" => months.rem_euclid(12) as f64,
        "MD" if to.2 >= from.2 => f64::from(to.2 - from.2),
        "MD" => {
            // The days from the start's day in the month before the end.
            let (year, month) = if to.1 == 1 { (to.0 - 1, 12) } else { (to.0, to.1 - 1) };
            f64::from(days_in_month(year, month).saturating_sub(from.2) + to.2)
        }
        "YD" => {
            // The start moved to the latest year that keeps it on or before the end.
            let year = if (to.1, to.2) >= (from.1, from.2) { to.0 } else { to.0 - 1 };
            let day = from.2.min(days_in_month(year, from.1));
            last - date_serial(year, from.1, day).ok_or(CellError::Num)?
        }
        _ => return Err(CellError::Num),
    })
}
