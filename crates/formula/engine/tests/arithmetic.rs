mod common;

use common::{number, value_of};
use excel_model::CellValue;

#[test]
fn computes_with_precedence() {
    assert_eq!(value_of("=1+2*3"), number(7.0));
    assert_eq!(value_of("=(1+2)*3"), number(9.0));
    assert_eq!(value_of("=2^3^2"), number(64.0));
    assert_eq!(value_of("=-2^2"), number(4.0));
    assert_eq!(value_of("=50%"), number(0.5));
    assert_eq!(value_of("=10-2-3"), number(5.0));
}

#[test]
fn text_with_a_number_converts_in_arithmetic() {
    assert_eq!(value_of("=\"3\"+1"), number(4.0));
}

#[test]
fn booleans_are_one_and_zero_in_arithmetic() {
    assert_eq!(value_of("=TRUE+1"), number(2.0));
}

#[test]
fn empty_cells_are_zero_in_arithmetic() {
    assert_eq!(value_of("=B50+1"), number(1.0));
}

#[test]
fn concatenates_text() {
    assert_eq!(value_of("=\"사과\"&\" \"&A1"), CellValue::Text("사과 10".into()));
}

#[test]
fn comparisons() {
    assert_eq!(value_of("=A1>A2"), CellValue::Bool(true));
    assert_eq!(value_of("=A1<=A2"), CellValue::Bool(false));
    assert_eq!(value_of("=A1<>A2"), CellValue::Bool(true));
    assert_eq!(value_of("=\"a\"=\"A\""), CellValue::Bool(true));
    assert_eq!(value_of("=\"a\">5"), CellValue::Bool(true));
    assert_eq!(value_of("=TRUE>\"z\""), CellValue::Bool(true));
    assert_eq!(value_of("=B50=0"), CellValue::Bool(true));
    assert_eq!(value_of("=B50=\"\""), CellValue::Bool(true));
}

#[test]
fn functions_work_through_the_engine() {
    assert_eq!(value_of("=SUM(A1:A2)"), number(14.0));
    assert_eq!(value_of("=AVERAGE(A1:A2)"), number(7.0));
    assert_eq!(value_of("=COUNT(A1:A9)"), number(2.0));
    assert_eq!(value_of("=MIN(A1,A2)"), number(4.0));
    assert_eq!(value_of("=MAX(A1:A2,100)"), number(100.0));
    assert_eq!(value_of("=IF(A1>5,\"big\",\"small\")"), CellValue::Text("big".into()));
    assert_eq!(value_of("=sum(a1:a2)"), number(14.0));
}

#[test]
fn functions_nest() {
    assert_eq!(value_of("=IF(SUM(A1:A2)>10,MAX(A1,A2)*2,0)"), number(20.0));
}
