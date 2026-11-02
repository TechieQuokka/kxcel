pub(crate) mod date_fn;
pub(crate) mod datedif;
pub(crate) mod day;
pub(crate) mod month;
pub(crate) mod today;
pub(crate) mod year;

use excel_model::{CellError, date_serial, serial_date};

use crate::{Arg, scalar_number};

/// The year, month, and day of a date argument, or `#NUM!` outside the dates
/// that can be shown.
pub(crate) fn parts(arg: &Arg) -> Result<(i64, u32, u32), CellError> {
    serial_date(scalar_number(arg)?).ok_or(CellError::Num)
}

/// The serial number of a year, month, and day that may overflow, as `DATE`
/// allows: month 13 is January of the next year, and day 0 is the last day
/// of the month before. Years below 1900 count from 1900, as in Excel.
pub(crate) fn normalized(year: f64, month: f64, day: f64) -> Result<f64, CellError> {
    let (year, month, day) = (year.trunc(), month.trunc(), day.trunc());
    if !(0.0..10_000.0).contains(&year) || month.abs() > 1e6 || day.abs() > 1e8 {
        return Err(CellError::Num);
    }
    let year = if year < 1900.0 { year + 1900.0 } else { year };
    let months = year as i64 * 12 + month as i64 - 1;
    let first = date_serial(months.div_euclid(12), months.rem_euclid(12) as u32 + 1, 1).ok_or(CellError::Num)?;
    let serial = first + day - 1.0;
    serial_date(serial).map(|_| serial).ok_or(CellError::Num)
}

/// How many days a month has.
pub(crate) fn days_in_month(year: i64, month: u32) -> u32 {
    (28..=31).rev().find(|day| date_serial(year, month, *day).is_some()).unwrap_or(28)
}
