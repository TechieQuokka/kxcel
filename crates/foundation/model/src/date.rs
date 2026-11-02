//! Dates as Excel stores them: a serial number of days, where 1 is 1900-01-01.
//!
//! Excel counts 1900 as a leap year, so serial 60 is the nonexistent
//! 1900-02-29 and every later date is one higher than a plain count would be.
//! This keeps the same numbering, so files and formulas agree with Excel.

/// The last day Excel can show, 9999-12-31.
pub const MAX_DATE_SERIAL: f64 = 2_958_465.0;

/// Days from 1970-01-01 to a date, for any year. From Howard Hinnant's
/// `days_from_civil`.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The date `days` after 1970-01-01. The inverse of [`days_from_civil`].
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The serial number of a date from 1900-01-01 to 9999-12-31, or `None` for a
/// date that does not exist or is out of that range.
pub fn date_serial(year: i64, month: u32, day: u32) -> Option<f64> {
    if !(1900..=9999).contains(&year) || !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    let days = days_from_civil(year, month, day);
    // Before March 1900, Excel's count has not yet passed its extra leap day.
    let epoch = if (year, month) < (1900, 3) { days_from_civil(1899, 12, 31) } else { days_from_civil(1899, 12, 30) };
    Some((days - epoch) as f64)
}

/// The date a serial number falls on, ignoring any time of day, or `None`
/// outside 1900-01-01 to 9999-12-31. Serial 60 is Excel's 1900-02-29.
pub fn serial_date(serial: f64) -> Option<(i64, u32, u32)> {
    if !serial.is_finite() || !(1.0..MAX_DATE_SERIAL + 1.0).contains(&serial) {
        return None;
    }
    let days = serial.floor() as i64;
    Some(match days {
        60 => (1900, 2, 29),
        ..60 => civil_from_days(days_from_civil(1899, 12, 31) + days),
        _ => civil_from_days(days_from_civil(1899, 12, 30) + days),
    })
}

/// Shows a serial number as `YYYY-MM-DD`, or `None` outside the date range.
pub fn format_date(serial: f64) -> Option<String> {
    let (year, month, day) = serial_date(serial)?;
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

/// Reads a typed date such as `2026-11-02`, `2026/11/2`, or `2026.11.02`,
/// giving its serial number. The year comes first and has four digits.
pub fn parse_date(text: &str) -> Option<f64> {
    let separator = text.chars().find(|c| matches!(c, '-' | '/' | '.'))?;
    let parts: Vec<&str> = text.split(separator).collect();
    let [year, month, day] = parts[..] else {
        return None;
    };
    let digits = |part: &str, lengths: std::ops::RangeInclusive<usize>| {
        (lengths.contains(&part.len()) && part.bytes().all(|b| b.is_ascii_digit())).then(|| part.parse::<u32>().ok())?
    };
    date_serial(i64::from(digits(year, 4..=4)?), digits(month, 1..=2)?, digits(day, 1..=2)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_excel_serial_numbers() {
        for (date, serial) in [
            ((1900, 1, 1), 1.0),
            ((1900, 2, 28), 59.0),
            ((1900, 3, 1), 61.0),
            ((1970, 1, 1), 25_569.0),
            ((2000, 2, 29), 36_585.0),
            ((2026, 11, 2), 46_328.0),
            ((9999, 12, 31), MAX_DATE_SERIAL),
        ] {
            assert_eq!(date_serial(date.0, date.1, date.2), Some(serial), "{date:?}");
            assert_eq!(serial_date(serial), Some(date), "{serial}");
        }
        assert_eq!(serial_date(60.0), Some((1900, 2, 29)), "Excel's leap day");
    }

    #[test]
    fn every_day_round_trips() {
        for serial in (61..=MAX_DATE_SERIAL as i64).step_by(97) {
            let (y, m, d) = serial_date(serial as f64).unwrap();
            assert_eq!(date_serial(y, m, d), Some(serial as f64));
        }
    }

    #[test]
    fn rejects_dates_that_do_not_exist() {
        assert_eq!(date_serial(2026, 2, 29), None);
        assert_eq!(date_serial(2026, 13, 1), None);
        assert_eq!(date_serial(1899, 12, 31), None);
        assert_eq!(serial_date(0.0), None);
        assert_eq!(serial_date(-3.0), None);
        assert_eq!(serial_date(MAX_DATE_SERIAL + 1.0), None);
    }

    #[test]
    fn parses_typed_dates() {
        assert_eq!(parse_date("2026-11-02"), Some(46_328.0));
        assert_eq!(parse_date("2026/11/2"), Some(46_328.0));
        assert_eq!(parse_date("2026.11.02"), Some(46_328.0));
        for text in ["2026-11", "26-11-02", "2026-11-02-1", "2026-1x-02", "2026-02-30", "2026-11/02", "-1-1"] {
            assert_eq!(parse_date(text), None, "{text}");
        }
    }

    #[test]
    fn formats_the_day_and_ignores_the_time() {
        assert_eq!(format_date(46_328.75).as_deref(), Some("2026-11-02"));
        assert_eq!(format_date(0.5), None);
    }
}
