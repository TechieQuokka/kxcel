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
