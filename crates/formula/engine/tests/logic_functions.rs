mod common;

use common::{calculated, number};
use excel_model::{CellError, CellValue};

fn result(formula: &str) -> CellValue {
    let cells = [("A1", "10"), ("A2", "사과"), ("A3", "=1/0"), ("A4", "TRUE"), ("A5", "0"), ("Z99", formula)];
    calculated(&cells).active().value("Z99".parse().unwrap()).clone()
}

fn yes(b: bool) -> CellValue {
    CellValue::Bool(b)
}

#[test]
fn and_or_not() {
    for (formula, expected) in [
        ("=AND(1<2, 3>2)", yes(true)),
        ("=AND(1<2, 3<2)", yes(false)),
        ("=OR(1>2, 3>2)", yes(true)),
        ("=OR(1>2, 3<2)", yes(false)),
        ("=AND(A1:A2, A4)", yes(true)),
        ("=OR(A5, A2)", yes(false)),
        ("=NOT(1>2)", yes(true)),
        ("=NOT(A1)", yes(false)),
    ] {
        assert_eq!(result(formula), expected, "{formula}");
    }
    assert_eq!(result("=AND(A2)"), CellValue::Error(CellError::Value), "nothing to combine");
    assert_eq!(result("=OR(A3, TRUE)"), CellValue::Error(CellError::Div0));
    assert_eq!(result("=AND(\"x\")"), CellValue::Error(CellError::Value));
}

#[test]
fn error_handling() {
    for (formula, expected) in [
        ("=IFERROR(1/0, 0)", number(0.0)),
        ("=IFERROR(A3, \"없음\")", CellValue::Text("없음".into())),
        ("=IFERROR(A1*2, 0)", number(20.0)),
        ("=IFERROR(A1:A2, 1)", number(1.0)),
        ("=ISERROR(A3)", yes(true)),
        ("=ISERROR(A1)", yes(false)),
    ] {
        assert_eq!(result(formula), expected, "{formula}");
    }
}

#[test]
fn several_conditions() {
    let grade = |score: &str| {
        let cells = [("A1", score), ("B1", "=IFS(A1>=90, \"A\", A1>=80, \"B\", TRUE, \"C\")")];
        calculated(&cells).active().value("B1".parse().unwrap()).to_string()
    };
    assert_eq!([grade("95"), grade("85"), grade("10")], ["A", "B", "C"]);
    assert_eq!(result("=IFS(A1>100, 1)"), CellValue::Error(CellError::Na), "none is true");
    assert_eq!(result("=IFS(A3, 1)"), CellValue::Error(CellError::Div0));
    assert_eq!(result("=IFS(TRUE)"), CellValue::Error(CellError::Value));
}

#[test]
fn value_kinds() {
    for (formula, expected) in [
        ("=ISBLANK(B9)", true),
        ("=ISBLANK(A5)", false),
        ("=ISBLANK(\"\")", false),
        ("=ISNUMBER(A1)", true),
        ("=ISNUMBER(\"5\")", false),
        ("=ISNUMBER(B9)", false),
        ("=ISTEXT(A2)", true),
        ("=ISTEXT(A1)", false),
    ] {
        assert_eq!(result(formula), yes(expected), "{formula}");
    }
}

#[test]
fn ifna_replaces_only_not_found() {
    let cells = [("A1", "사과"), ("B1", "3"), ("Z99", "")];
    let lookup = |formula: &str| {
        let mut all = cells.to_vec();
        all[2].1 = formula;
        calculated(&all).active().value("Z99".parse().unwrap()).clone()
    };
    assert_eq!(lookup("=IFNA(VLOOKUP(\"사과\", A1:B1, 2, FALSE), 0)"), number(3.0));
    assert_eq!(lookup("=IFNA(VLOOKUP(\"배\", A1:B1, 2, FALSE), \"없음\")"), CellValue::Text("없음".into()));
    assert_eq!(result("=IFNA(A3, 0)"), CellValue::Error(CellError::Div0), "other errors stay");
    assert_eq!(result("=IFNA(A1, 0)"), number(10.0));
    assert_eq!(result("=IFNA(A1)"), CellValue::Error(CellError::Value));
}
