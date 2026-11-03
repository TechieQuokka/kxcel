mod common;

use common::{calculated, number};
use excel_model::{CellError, CellValue, NumberFormat};

fn shown(formula: &str) -> String {
    calculated(&[("A1", "2026-11-02"), ("Z99", formula)]).active().display("Z99".parse().unwrap())
}

fn result(formula: &str) -> CellValue {
    calculated(&[("A1", "2026-11-02"), ("Z99", formula)]).active().value("Z99".parse().unwrap()).clone()
}

#[test]
fn building_dates() {
    for (formula, expected) in [
        ("=DATE(2026, 11, 2)", "2026-11-02"),
        ("=DATE(2026, 13, 1)", "2027-01-01"),
        ("=DATE(2026, 3, 0)", "2026-02-28"),
        ("=DATE(2024, 2, 29)", "2024-02-29"),
        ("=DATE(2026, 0, 1)", "2025-12-01"),
        ("=DATE(2026, 1, 1) + 30", "2026-01-31"),
        ("=DATE(26, 1, 1)", "1926-01-01"),
    ] {
        assert_eq!(shown(formula), expected, "{formula}");
    }
    assert_eq!(result("=DATE(10000, 1, 1)"), CellValue::Error(CellError::Num));
}

#[test]
fn parts_of_dates() {
    assert_eq!(result("=YEAR(A1)"), number(2026.0));
    assert_eq!(result("=MONTH(A1)"), number(11.0));
    assert_eq!(result("=DAY(A1)"), number(2.0));
    assert_eq!(result("=YEAR(-1)"), CellValue::Error(CellError::Num));
    assert_eq!(shown("=YEAR(A1)"), "2026", "a part is a plain number, not a date");
}

#[test]
fn time_between_dates() {
    let between =
        |start: &str, end: &str, unit: &str| result(&format!("=DATEDIF(DATE({start}), DATE({end}), \"{unit}\")"));
    assert_eq!(between("2000, 5, 10", "2026, 11, 2", "Y"), number(26.0));
    assert_eq!(between("2000, 11, 3", "2026, 11, 2", "Y"), number(25.0), "a day short of the birthday");
    assert_eq!(between("2026, 1, 31", "2026, 3, 1", "M"), number(1.0));
    assert_eq!(between("2026, 1, 1", "2026, 11, 2", "D"), number(305.0));
    assert_eq!(between("2025, 3, 15", "2026, 11, 2", "YM"), number(7.0));
    assert_eq!(between("2026, 1, 15", "2026, 3, 10", "MD"), number(23.0));
    assert_eq!(between("2025, 12, 25", "2026, 11, 2", "YD"), number(312.0));
    assert_eq!(between("2026, 1, 1", "2025, 1, 1", "D"), CellValue::Error(CellError::Num));
    assert_eq!(between("2026, 1, 1", "2026, 2, 1", "W"), CellValue::Error(CellError::Num));
}

#[test]
fn today_is_a_date_that_stays_current() {
    let book = calculated(&[("A1", "=TODAY()"), ("B1", "=TODAY()-A1")]);
    let today = book.active().get("A1".parse().unwrap()).unwrap();
    assert_eq!(today.format, NumberFormat::Date);
    let serial = today.value.as_number().unwrap();
    assert!(serial > 46_000.0, "{serial}");
    assert_eq!(book.active().value("B1".parse().unwrap()), &number(0.0));
    assert!(excel_formula_engine::FormulaEngine::default().functions().iter().any(|(name, _)| *name == "TODAY"));
}

#[test]
fn months_later_and_month_ends() {
    for (formula, expected) in [
        ("=EDATE(A1, 3)", "2027-02-02"),
        ("=EDATE(A1, -12)", "2025-11-02"),
        ("=EDATE(A1, 0)", "2026-11-02"),
        ("=EDATE(DATE(2026, 1, 31), 1)", "2026-02-28"),
        ("=EDATE(DATE(2024, 1, 31), 1)", "2024-02-29"),
        ("=EDATE(DATE(2026, 3, 31), 1.9)", "2026-04-30"),
        ("=EOMONTH(A1, 0)", "2026-11-30"),
        ("=EOMONTH(A1, 3)", "2027-02-28"),
        ("=EOMONTH(DATE(2026, 3, 15), -1)", "2026-02-28"),
        ("=EOMONTH(DATE(2026, 12, 31), 1)", "2027-01-31"),
    ] {
        assert_eq!(shown(formula), expected, "{formula}");
    }
    assert_eq!(result("=EDATE(DATE(9999, 12, 1), 1)"), CellValue::Error(CellError::Num));
    assert_eq!(result("=EOMONTH(-1, 0)"), CellValue::Error(CellError::Num));
    assert_eq!(result("=EDATE(A1)"), CellValue::Error(CellError::Value));
}

#[test]
fn days_of_the_week() {
    // 2026-11-02 is a Monday, and 2026-11-08 a Sunday.
    for (formula, expected) in [
        ("=WEEKDAY(A1)", 2.0),
        ("=WEEKDAY(A1, 1)", 2.0),
        ("=WEEKDAY(A1, 2)", 1.0),
        ("=WEEKDAY(A1, 3)", 0.0),
        ("=WEEKDAY(A1 + 6)", 1.0),
        ("=WEEKDAY(A1 + 6, 2)", 7.0),
        ("=WEEKDAY(A1 + 5, 3)", 5.0),
        ("=WEEKDAY(DATE(2000, 1, 1))", 7.0),
    ] {
        assert_eq!(result(formula), number(expected), "{formula}");
    }
    assert_eq!(result("=WEEKDAY(A1, 4)"), CellValue::Error(CellError::Num));
    assert_eq!(result("=WEEKDAY(-1)"), CellValue::Error(CellError::Num));
    assert_eq!(shown("=WEEKDAY(A1)"), "2", "a number, not a date");
}

#[test]
fn working_days() {
    let days = |formula: &str| {
        let cells = [("A1", "2026-11-02"), ("H1", "2026-11-03"), ("H2", "2026-11-07"), ("H3", ""), ("Z99", formula)];
        let book = calculated(&cells);
        (book.active().value("Z99".parse().unwrap()).clone(), book.active().display("Z99".parse().unwrap()))
    };
    for (formula, expected) in [
        ("=NETWORKDAYS(A1, A1 + 11)", "10"),
        ("=NETWORKDAYS(A1, A1 + 11, H1:H3)", "9"),
        ("=NETWORKDAYS(A1, A1 + 11, H1)", "9"),
        ("=NETWORKDAYS(A1 + 11, A1)", "-10"),
        ("=NETWORKDAYS(A1 + 5, A1 + 6)", "0"),
        ("=NETWORKDAYS(DATE(2026, 1, 1), DATE(2026, 12, 31))", "261"),
        ("=WORKDAY(A1 + 4, 1)", "2026-11-09"),
        ("=WORKDAY(A1, 10)", "2026-11-16"),
        ("=WORKDAY(A1, 1, H1:H3)", "2026-11-04"),
        ("=WORKDAY(A1, -1)", "2026-10-30"),
        ("=WORKDAY(A1 + 5, 0)", "2026-11-07"),
        ("=WORKDAY(A1, 2.9)", "2026-11-04"),
    ] {
        assert_eq!(days(formula).1, expected, "{formula}");
    }
    assert_eq!(days("=NETWORKDAYS(A1, A1 + 1, \"x\")").0, CellValue::Error(CellError::Value));
    assert_eq!(days("=WORKDAY(DATE(9999, 12, 31), 1)").0, CellValue::Error(CellError::Num));
    assert_eq!(days("=WORKDAY(A1)").0, CellValue::Error(CellError::Value));
}

#[test]
fn days_between() {
    assert_eq!(result("=DAYS(DATE(2026, 12, 25), A1)"), number(53.0));
    assert_eq!(result("=DAYS(A1, DATE(2026, 12, 25))"), number(-53.0));
    assert_eq!(result("=DAYS(A1 + 0.9, A1)"), number(0.0));
    assert_eq!(shown("=DAYS(A1 + 1000, A1)"), "1000", "a count, not a date");
    assert_eq!(result("=DAYS(\"x\", A1)"), CellValue::Error(CellError::Value));
}
