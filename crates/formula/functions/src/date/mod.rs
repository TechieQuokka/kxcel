pub(crate) mod date_fn;
pub(crate) mod datedif;
pub(crate) mod day;
pub(crate) mod days;
pub(crate) mod edate;
pub(crate) mod month;
pub(crate) mod networkdays;
pub(crate) mod today;
pub(crate) mod weekday_fn;
pub(crate) mod workday;
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

/// The day of the week of a serial number, 0 for Sunday to 6 for Saturday.
pub(crate) fn weekday(serial: f64) -> u32 {
    // Serial 1, 1900-01-01, is a Sunday in Excel's count.
    ((serial.floor() as i64 - 1).rem_euclid(7)) as u32
}

/// Whether a serial number falls on Monday to Friday.
pub(crate) fn is_weekday(serial: f64) -> bool {
    !matches!(weekday(serial), 0 | 6)
}

/// The days of an optional holidays argument, as whole serial numbers.
/// Empty cells are skipped, and anything else that is not a date is `#VALUE!`.
pub(crate) fn holidays(arg: Option<&Arg>) -> Result<std::collections::HashSet<i64>, CellError> {
    let mut days = std::collections::HashSet::new();
    let mut add = |value: &excel_model::CellValue| match value {
        excel_model::CellValue::Number(n) => {
            days.insert(n.floor() as i64);
            Ok(())
        }
        excel_model::CellValue::Empty => Ok(()),
        excel_model::CellValue::Error(error) => Err(*error),
        _ => Err(CellError::Value),
    };
    match arg {
        None => {}
        Some(Arg::Cells(cells)) => cells.values().try_for_each(&mut add)?,
        Some(Arg::Value(value)) => add(value)?,
    }
    Ok(days)
}

/// The date `months` after a date, keeping its day where the month has it and
/// using the month's last day otherwise, as a serial number.
pub(crate) fn months_later(date: &Arg, months: &Arg, last_day: bool) -> Result<f64, CellError> {
    let (year, month, day) = parts(date)?;
    let months = scalar_number(months)?.trunc();
    if months.abs() > 1e6 {
        return Err(CellError::Num);
    }
    let total = year * 12 + i64::from(month) - 1 + months as i64;
    let (year, month) = (total.div_euclid(12), total.rem_euclid(12) as u32 + 1);
    let last = days_in_month(year, month);
    let day = if last_day { last } else { day.min(last) };
    date_serial(year, month, day).ok_or(CellError::Num)
}
