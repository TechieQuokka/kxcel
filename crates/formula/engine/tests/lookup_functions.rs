mod common;

use common::{calculated, number};
use excel_model::{CellError, CellValue};

/// A price list: name, price, and stock, sorted by price in column B.
const TABLE: [(&str, &str); 12] = [
    ("A1", "배"),
    ("B1", "1000"),
    ("C1", "5"),
    ("A2", "사과"),
    ("B2", "2000"),
    ("C2", "0"),
    ("A3", "Apple"),
    ("B3", "3000"),
    ("C3", "12"),
    ("A4", "감"),
    ("B4", "4000"),
    ("E1", "사과"),
];

fn result(formula: &str) -> CellValue {
    let mut cells = TABLE.to_vec();
    cells.push(("Z99", formula));
    calculated(&cells).active().value("Z99".parse().unwrap()).clone()
}

fn text(s: &str) -> CellValue {
    CellValue::Text(s.into())
}

fn error(error: CellError) -> CellValue {
    CellValue::Error(error)
}

#[test]
fn vlookup_and_hlookup() {
    for (formula, expected) in [
        ("=VLOOKUP(\"사과\", A1:C4, 2, FALSE)", number(2000.0)),
        ("=VLOOKUP(E1, A1:C4, 3, FALSE)", number(0.0)),
        ("=VLOOKUP(\"apple\", A1:C4, 2, FALSE)", number(3000.0)),
        ("=VLOOKUP(\"사*\", A1:C4, 2, FALSE)", number(2000.0)),
        ("=VLOOKUP(\"감\", A1:C4, 3, FALSE)", number(0.0)),
        ("=VLOOKUP(\"귤\", A1:C4, 2, FALSE)", error(CellError::Na)),
        ("=VLOOKUP(2500, B1:C4, 2)", number(0.0)),
        ("=VLOOKUP(3000, B1:C4, 2, TRUE)", number(12.0)),
        ("=VLOOKUP(500, B1:C4, 2)", error(CellError::Na)),
        ("=VLOOKUP(\"사과\", A1:C4, 4, FALSE)", error(CellError::Ref)),
        ("=VLOOKUP(\"사과\", A1:C4, 0, FALSE)", error(CellError::Value)),
        ("=HLOOKUP(2000, B1:B4, 1, FALSE)", error(CellError::Na)),
        ("=HLOOKUP(\"배\", A1:C2, 2, FALSE)", text("사과")),
    ] {
        assert_eq!(result(formula), expected, "{formula}");
    }
}

#[test]
fn index_and_match() {
    for (formula, expected) in [
        ("=INDEX(A1:A4, 2)", text("사과")),
        ("=INDEX(A1:C4, 3, 3)", number(12.0)),
        ("=INDEX(A1:C1, 2)", number(1000.0)),
        ("=INDEX(A1:A4, 9)", error(CellError::Ref)),
        ("=INDEX(A1:C4, 2)", error(CellError::Ref)),
        ("=MATCH(\"감\", A1:A4, 0)", number(4.0)),
        ("=MATCH(2500, B1:B4)", number(2.0)),
        ("=MATCH(2500, B1:B4, 1)", number(2.0)),
        ("=MATCH(\"귤\", A1:A4, 0)", error(CellError::Na)),
        ("=MATCH(1, A1:C4, 0)", error(CellError::Na)),
        ("=INDEX(B1:B4, MATCH(\"사과\", A1:A4, 0))", number(2000.0)),
    ] {
        assert_eq!(result(formula), expected, "{formula}");
    }
}

#[test]
fn xlookup() {
    for (formula, expected) in [
        ("=XLOOKUP(\"사과\", A1:A4, B1:B4)", number(2000.0)),
        ("=XLOOKUP(\"귤\", A1:A4, B1:B4)", error(CellError::Na)),
        ("=XLOOKUP(\"귤\", A1:A4, B1:B4, \"없음\")", text("없음")),
        ("=XLOOKUP(2500, B1:B4, A1:A4, \"\", -1)", text("사과")),
        ("=XLOOKUP(2500, B1:B4, A1:A4, \"\", 1)", text("Apple")),
        ("=XLOOKUP(\"사*\", A1:A4, B1:B4)", error(CellError::Na)),
        ("=XLOOKUP(\"사*\", A1:A4, B1:B4, \"\", 2)", number(2000.0)),
        ("=XLOOKUP(\"배\", A1:C1, A2:C2)", text("사과")),
        ("=XLOOKUP(1, A1:A4, B1:B3)", error(CellError::Value)),
    ] {
        assert_eq!(result(formula), expected, "{formula}");
    }
}
