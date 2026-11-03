mod common;

use common::{calculated, number, value_of};
use excel_model::{CellError, CellValue};

fn error(error: CellError) -> CellValue {
    CellValue::Error(error)
}

#[test]
fn rounding() {
    for (formula, expected) in [
        ("=ROUND(2.675, 2)", 2.68),
        ("=ROUND(-1.5, 0)", -2.0),
        ("=ROUND(1234.5, -2)", 1200.0),
        ("=ROUND(0.1+0.2, 15)", 0.3),
        ("=ROUNDUP(3.21, 1)", 3.3),
        ("=ROUNDUP(-3.21, 1)", -3.3),
        ("=ROUNDUP(3.2, 0)", 4.0),
        ("=ROUNDDOWN(3.99, 1)", 3.9),
        ("=ROUNDDOWN(-3.99, 0)", -3.0),
        ("=ROUNDDOWN(1999, -3)", 1000.0),
        ("=INT(7.9)", 7.0),
        ("=INT(-7.1)", -8.0),
    ] {
        assert_eq!(value_of(formula), number(expected), "{formula}");
    }
}

#[test]
fn arithmetic_functions() {
    for (formula, expected) in [
        ("=ABS(-4.5)", number(4.5)),
        ("=MOD(10, 3)", number(1.0)),
        ("=MOD(-10, 3)", number(2.0)),
        ("=MOD(10, -3)", number(-2.0)),
        ("=MOD(5, 0)", error(CellError::Div0)),
        ("=POWER(2, 10)", number(1024.0)),
        ("=POWER(-1, 0.5)", error(CellError::Num)),
        ("=SQRT(16)", number(4.0)),
        ("=SQRT(-1)", error(CellError::Num)),
        ("=PRODUCT(A1:A2, 2)", number(80.0)),
        ("=PRODUCT(Z1:Z5)", number(0.0)),
        ("=ABS(A1:A2)", error(CellError::Value)),
        ("=ABS(\"x\")", error(CellError::Value)),
        ("=ROUND(1)", error(CellError::Value)),
    ] {
        assert_eq!(value_of(formula), expected, "{formula}");
    }
}

fn result(cells: &[(&str, &str)], formula: &str) -> CellValue {
    let mut all = cells.to_vec();
    all.push(("Z99", formula));
    calculated(&all).active().value("Z99".parse().unwrap()).clone()
}

const FRUIT: [(&str, &str); 10] = [
    ("A1", "사과"),
    ("B1", "3"),
    ("A2", "배"),
    ("B2", "5"),
    ("A3", "사과"),
    ("B3", "10"),
    ("A4", "Apple"),
    ("B4", "x"),
    ("A5", "apple pie"),
    ("B5", "7"),
];

#[test]
fn counting() {
    for (formula, expected) in [
        ("=COUNTA(A1:B6)", 10.0),
        ("=COUNTA(1, \"a\", TRUE)", 3.0),
        ("=COUNTBLANK(A1:B6)", 2.0),
        ("=COUNTBLANK(C1:C1000)", 1000.0),
        ("=COUNTIF(A1:A5, \"사과\")", 2.0),
        ("=COUNTIF(A1:A5, \"apple*\")", 2.0),
        ("=COUNTIF(B1:B5, \">4\")", 3.0),
        ("=COUNTIF(B1:B6, \"\")", 1.0),
        ("=COUNTIF(B1:B6, \"<>\")", 5.0),
        ("=COUNTIF(A1:A100, \"?\")", 1.0),
        ("=COUNTIFS(A1:A5, \"사과\", B1:B5, \">5\")", 1.0),
        ("=COUNTIFS(A1:A6, \"\", B1:B6, \"\")", 1.0),
    ] {
        assert_eq!(result(&FRUIT, formula), number(expected), "{formula}");
    }
}

#[test]
fn conditional_sums_and_averages() {
    for (formula, expected) in [
        ("=SUMIF(A1:A5, \"사과\", B1:B5)", number(13.0)),
        ("=SUMIF(B1:B5, \">4\")", number(22.0)),
        ("=SUMIF(A1:A100, \"<>사과\", B1:B100)", number(12.0)),
        ("=SUMIFS(B1:B5, A1:A5, \"사과\", B1:B5, \"<5\")", number(3.0)),
        ("=AVERAGEIF(A1:A5, \"사과\", B1:B5)", number(6.5)),
        ("=AVERAGEIF(A1:A5, \"없음\", B1:B5)", error(CellError::Div0)),
        ("=SUMIFS(B1:B5, A1:A4, \"사과\")", error(CellError::Value)),
        ("=SUMIF(5, \">1\")", error(CellError::Value)),
    ] {
        assert_eq!(result(&FRUIT, formula), expected, "{formula}");
    }
}

#[test]
fn conditions_read_from_cells_and_errors_in_summed_cells() {
    let cells = [("A1", "a"), ("B1", "1"), ("A2", "b"), ("B2", "=1/0"), ("C1", "a")];
    assert_eq!(result(&cells, "=SUMIF(A1:A2, C1, B1:B2)"), number(1.0), "a skipped error does not matter");
    assert_eq!(result(&cells, "=SUMIF(A1:A2, \"b\", B1:B2)"), error(CellError::Div0));
}

#[test]
fn conditions_follow_their_cells() {
    // A conditional sum reads its whole range, so a later edit there recalculates it.
    let book = calculated(&[("A1", "1"), ("A2", "2"), ("B1", "=SUMIF(A1:A3, \">1\")")]);
    assert_eq!(book.active().value("B1".parse().unwrap()), &number(2.0));
}

#[test]
fn conditional_averages_and_extremes_with_several_conditions() {
    for (formula, expected) in [
        ("=AVERAGEIFS(B1:B5, A1:A5, \"사과\")", number(6.5)),
        ("=AVERAGEIFS(B1:B5, A1:A5, \"사과\", B1:B5, \">5\")", number(10.0)),
        ("=AVERAGEIFS(B1:B5, A1:A5, \"없음\")", error(CellError::Div0)),
        ("=MAXIFS(B1:B5, A1:A5, \"사과\")", number(10.0)),
        ("=MAXIFS(B1:B5, A1:A5, \"사과\", B1:B5, \"<5\")", number(3.0)),
        ("=MINIFS(B1:B5, A1:A5, \"사과\")", number(3.0)),
        ("=MAXIFS(B1:B5, A1:A5, \"없음\")", number(0.0)),
        ("=MINIFS(B1:B5, A1:A5, \"apple\")", number(0.0)),
        ("=MAXIFS(B1:B5, A1:A4, \"사과\")", error(CellError::Value)),
        ("=MINIFS(B1:B5, A1:A5)", error(CellError::Value)),
        ("=AVERAGEIFS(5, A1:A5, \"사과\")", error(CellError::Value)),
    ] {
        assert_eq!(result(&FRUIT, formula), expected, "{formula}");
    }
    let cells = [("A1", "a"), ("B1", "1"), ("A2", "b"), ("B2", "=1/0")];
    assert_eq!(result(&cells, "=MAXIFS(B1:B2, A1:A2, \"a\")"), number(1.0), "a skipped error does not matter");
    assert_eq!(result(&cells, "=MINIFS(B1:B2, A1:A2, \"b\")"), error(CellError::Div0));
}

#[test]
fn sums_of_products() {
    let cells = [
        ("A1", "1000"),
        ("B1", "3"),
        ("A2", "2500"),
        ("B2", "2"),
        ("A3", "메모"),
        ("B3", "9"),
        ("A4", "500"),
        ("C1", "=1/0"),
    ];
    for (formula, expected) in [
        ("=SUMPRODUCT(A1:A4, B1:B4)", number(8000.0)),
        ("=SUMPRODUCT(A1:A2)", number(3500.0)),
        ("=SUMPRODUCT(A1:B2, A1:B2)", number(1000.0 * 1000.0 + 9.0 + 2500.0 * 2500.0 + 4.0)),
        ("=SUMPRODUCT(2, 3)", number(6.0)),
        ("=SUMPRODUCT(A1:A4, B1:B3)", error(CellError::Value)),
        ("=SUMPRODUCT(A1:A2, C1:C2)", error(CellError::Div0)),
        ("=SUMPRODUCT(A1:A1000000, B1:B1000000)", number(8000.0)),
    ] {
        assert_eq!(result(&cells, formula), expected, "{formula}");
    }
}

#[test]
fn rounding_to_multiples() {
    for (formula, expected) in [
        ("=CEILING(12345, 100)", number(12400.0)),
        ("=CEILING(12345, 500)", number(12500.0)),
        ("=CEILING(12400, 100)", number(12400.0)),
        ("=CEILING(2.1)", number(3.0)),
        ("=CEILING(1.1, 0.1)", number(1.1)),
        ("=CEILING(0.7, 0.1)", number(0.7)),
        ("=CEILING(-2.5, 2)", number(-2.0)),
        ("=CEILING(-2.5, -2)", number(-4.0)),
        ("=CEILING(5, 0)", number(0.0)),
        ("=CEILING(2.5, -2)", error(CellError::Num)),
        ("=FLOOR(12345, 1000)", number(12000.0)),
        ("=FLOOR(2.9)", number(2.0)),
        ("=FLOOR(1.1, 0.1)", number(1.1)),
        ("=FLOOR(-2.5, 2)", number(-4.0)),
        ("=FLOOR(-2.5, -2)", number(-2.0)),
        ("=FLOOR(5, 0)", error(CellError::Div0)),
        ("=FLOOR(2.5, -2)", error(CellError::Num)),
        ("=FLOOR(\"x\", 2)", error(CellError::Value)),
    ] {
        assert_eq!(value_of(formula), expected, "{formula}");
    }
}

#[test]
fn statistics() {
    let cells = [
        ("A1", "30"),
        ("A2", "10"),
        ("A3", "20"),
        ("A4", "20"),
        ("A5", "메모"),
        ("A6", "100"),
        ("B1", "=1/0"),
        ("C1", "5"),
    ];
    for (formula, expected) in [
        ("=MEDIAN(A1:A6)", number(20.0)),
        ("=MEDIAN(A1:A3)", number(20.0)),
        ("=MEDIAN(1, 2, 3, 100)", number(2.5)),
        ("=MEDIAN(A5)", error(CellError::Num)),
        ("=LARGE(A1:A6, 1)", number(100.0)),
        ("=LARGE(A1:A6, 2)", number(30.0)),
        ("=LARGE(A1:A6, 3.9)", number(20.0)),
        ("=LARGE(A1:A6, 6)", error(CellError::Num)),
        ("=LARGE(A1:A6, 0)", error(CellError::Num)),
        ("=SMALL(A1:A6, 1)", number(10.0)),
        ("=SMALL(A1:A6, 5)", number(100.0)),
        ("=LARGE(A1:B6, 1)", error(CellError::Div0)),
        ("=RANK(A6, A1:A6)", number(1.0)),
        ("=RANK(A3, A1:A6)", number(3.0)),
        ("=RANK(A4, A1:A6)", number(3.0)),
        ("=RANK(A2, A1:A6)", number(5.0)),
        ("=RANK(A2, A1:A6, 1)", number(1.0)),
        ("=RANK(A1, A1:A6, 1)", number(4.0)),
        ("=RANK(C1, A1:A6)", error(CellError::Na)),
        ("=RANK(5, 5)", error(CellError::Value)),
        ("=STDEV(2, 4, 6)", number(2.0)),
        ("=STDEV(A1:A4)", number((200.0f64 / 3.0).sqrt())),
        ("=STDEV(5)", error(CellError::Div0)),
    ] {
        assert_eq!(result(&cells, formula), expected, "{formula}");
    }
}
