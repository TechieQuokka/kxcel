mod common;

use common::{calculated, number};
use excel_model::{CellError, CellValue};

fn result(formula: &str) -> CellValue {
    let cells = [("A1", "사과"), ("A2", ""), ("A3", "배"), ("B1", "12.5"), ("B2", "=1/0"), ("Z99", formula)];
    calculated(&cells).active().value("Z99".parse().unwrap()).clone()
}

fn text(s: &str) -> CellValue {
    CellValue::Text(s.into())
}

#[test]
fn pieces_of_text() {
    for (formula, expected) in [
        ("=LEN(\"사과 주스\")", number(5.0)),
        ("=LEN(B1)", number(4.0)),
        ("=LEN(C9)", number(0.0)),
        ("=LEFT(\"사과나무\", 2)", text("사과")),
        ("=LEFT(\"사과\")", text("사")),
        ("=RIGHT(\"사과나무\", 2)", text("나무")),
        ("=RIGHT(\"ab\", 9)", text("ab")),
        ("=MID(\"가나다라\", 2, 2)", text("나다")),
        ("=MID(\"가나\", 5, 2)", text("")),
        ("=UPPER(\"apple 사과\")", text("APPLE 사과")),
        ("=LOWER(\"APPLE\")", text("apple")),
        ("=TRIM(\"  a   b \")", text("a b")),
    ] {
        assert_eq!(result(formula), expected, "{formula}");
    }
    for formula in ["=LEFT(\"a\", -1)", "=MID(\"a\", 0, 1)", "=LEN(A1:A3)"] {
        assert_eq!(result(formula), CellValue::Error(CellError::Value), "{formula}");
    }
    assert_eq!(result("=LEN(B2)"), CellValue::Error(CellError::Div0));
}

#[test]
fn joining_finding_and_replacing() {
    for (formula, expected) in [
        ("=CONCAT(A1, \" \", 3, \"개\")", text("사과 3개")),
        ("=CONCAT(A1:A3)", text("사과배")),
        ("=TEXTJOIN(\", \", TRUE, A1:A3)", text("사과, 배")),
        ("=TEXTJOIN(\"/\", FALSE, A1:A3)", text("사과//배")),
        ("=TEXTJOIN(\"-\", FALSE, 2026, 11, 2)", text("2026-11-2")),
        ("=FIND(\"과\", \"사과나무\")", number(2.0)),
        ("=FIND(\"-\", \"2026-11-02\", 6)", number(8.0)),
        ("=FIND(\"A\", \"abc\")", CellValue::Error(CellError::Value)),
        ("=SUBSTITUTE(\"2026-11-02\", \"-\", \"/\")", text("2026/11/02")),
        ("=SUBSTITUTE(\"a-b-c\", \"-\", \"+\", 2)", text("a-b+c")),
        ("=SUBSTITUTE(\"a-b\", \"-\", \"+\", 5)", text("a-b")),
    ] {
        assert_eq!(result(formula), expected, "{formula}");
    }
}

#[test]
fn formats_and_numbers() {
    for (formula, expected) in [
        ("=TEXT(1234.5, \"#,##0.00\")", text("1,234.50")),
        ("=TEXT(B1, \"0\")", text("13")),
        ("=TEXT(46328, \"yyyy년 m월 d일\")", text("2026년 11월 2일")),
        ("=VALUE(\"1,234.5\")", number(1234.5)),
        ("=VALUE(\"15%\")", number(0.15)),
        ("=VALUE(\"사과\")", CellValue::Error(CellError::Value)),
        ("=VALUE(B1)", number(12.5)),
    ] {
        assert_eq!(result(formula), expected, "{formula}");
    }
}

#[test]
fn searching_replacing_and_repeating() {
    for (formula, expected) in [
        ("=SEARCH(\"apple\", \"Big Apple\")", number(5.0)),
        ("=SEARCH(\"a?c\", \"xxABC\")", number(3.0)),
        ("=SEARCH(\"b*d\", \"abcde\")", number(2.0)),
        ("=SEARCH(\"과\", \"사과 사과\", 3)", number(5.0)),
        ("=SEARCH(\"~?\", \"why?\")", number(4.0)),
        ("=SEARCH(\"\", \"abc\", 2)", number(2.0)),
        ("=SEARCH(\"z\", \"abc\")", CellValue::Error(CellError::Value)),
        ("=SEARCH(\"a\", \"abc\", 9)", CellValue::Error(CellError::Value)),
        ("=REPLACE(\"010-1234-5678\", 5, 4, \"****\")", text("010-****-5678")),
        ("=REPLACE(\"가나다\", 2, 0, \"X\")", text("가X나다")),
        ("=REPLACE(\"가나다\", 9, 1, \"!\")", text("가나다!")),
        ("=REPLACE(\"가나다\", 2, 99, \"\")", text("가")),
        ("=REPLACE(\"가\", 0, 1, \"x\")", CellValue::Error(CellError::Value)),
        ("=REPT(\"★\", 3)", text("★★★")),
        ("=REPT(\"ab\", 2.9)", text("abab")),
        ("=REPT(\"a\", 0)", text("")),
        ("=REPT(\"a\", -1)", CellValue::Error(CellError::Value)),
        ("=REPT(\"ab\", 20000)", CellValue::Error(CellError::Value)),
        ("=REPT(B2, 2)", CellValue::Error(CellError::Div0)),
    ] {
        assert_eq!(result(formula), expected, "{formula}");
    }
}

#[test]
fn text_around_a_delimiter() {
    for (formula, expected) in [
        ("=TEXTBEFORE(\"kim@example.com\", \"@\")", text("kim")),
        ("=TEXTAFTER(\"kim@example.com\", \"@\")", text("example.com")),
        ("=TEXTBEFORE(\"서울-강남-역삼\", \"-\", 2)", text("서울-강남")),
        ("=TEXTAFTER(\"서울-강남-역삼\", \"-\", 2)", text("역삼")),
        ("=TEXTBEFORE(\"서울-강남-역삼\", \"-\", -1)", text("서울-강남")),
        ("=TEXTAFTER(\"서울-강남-역삼\", \"-\", -1)", text("역삼")),
        ("=TEXTAFTER(\"서울-강남-역삼\", \"-\", -2)", text("강남-역삼")),
        ("=TEXTAFTER(\"a, b, c\", \", \")", text("b, c")),
        ("=TEXTBEFORE(\"abc\", \"\")", text("")),
        ("=TEXTAFTER(\"abc\", \"\")", text("abc")),
        ("=TEXTBEFORE(\"aXb\", \"x\")", CellValue::Error(CellError::Na)),
        ("=TEXTBEFORE(\"a-b\", \"-\", 2)", CellValue::Error(CellError::Na)),
        ("=TEXTAFTER(\"a-b\", \"-\", -3)", CellValue::Error(CellError::Na)),
        ("=TEXTAFTER(\"a-b\", \"-\", 0)", CellValue::Error(CellError::Value)),
        ("=IFNA(TEXTBEFORE(A1, \"@\"), A1)", text("사과")),
    ] {
        assert_eq!(result(formula), expected, "{formula}");
    }
}
