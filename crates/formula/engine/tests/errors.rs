mod common;

use common::{calculated, number, value_of};
use excel_command::Calculator;
use excel_formula_engine::FormulaEngine;
use excel_model::{Cell, CellError, CellValue, Sheet, Workbook};

fn error(e: CellError) -> CellValue {
    CellValue::Error(e)
}

#[test]
fn division_by_zero() {
    assert_eq!(value_of("=1/0"), error(CellError::Div0));
    assert_eq!(value_of("=1/B50"), error(CellError::Div0));
}

#[test]
fn numeric_overflow_and_invalid_powers() {
    assert_eq!(value_of("=10^400"), error(CellError::Num));
    assert_eq!(value_of("=(-1)^0.5"), error(CellError::Num));
    assert_eq!(value_of("=0^0"), error(CellError::Num));
    assert_eq!(value_of("=0^-1"), error(CellError::Div0));
}

#[test]
fn text_in_arithmetic_is_a_value_error() {
    assert_eq!(value_of("=\"x\"+1"), error(CellError::Value));
}

#[test]
fn a_range_as_a_single_value_is_a_value_error() {
    assert_eq!(value_of("=A1:A2+1"), error(CellError::Value));
}

#[test]
fn errors_propagate_through_formulas() {
    let book = calculated(&[("A1", "=1/0"), ("A2", "=A1+1"), ("A3", "=SUM(A1:A2)")]);
    for addr in ["A2", "A3"] {
        assert_eq!(book.active().value(addr.parse().unwrap()), &error(CellError::Div0), "{addr}");
    }
}

#[test]
fn if_can_guard_against_an_error() {
    assert_eq!(value_of("=IF(B50=0,0,1/B50)"), number(0.0));
}

#[test]
fn an_unknown_sheet_is_a_ref_error() {
    assert_eq!(value_of("=Missing!A1"), error(CellError::Ref));
    assert_eq!(value_of("=SUM(Missing!A1:A2)"), error(CellError::Ref));
}

#[test]
fn a_ref_error_literal_stays_an_error() {
    assert_eq!(value_of("=#REF!+1"), error(CellError::Ref));
}

#[test]
fn circular_references_are_cycle_errors() {
    let book =
        calculated(&[("A1", "=B1+1"), ("B1", "=A1+1"), ("C1", "=A1"), ("D1", "=D1"), ("E1", "1"), ("F1", "=E1")]);
    let at = |addr: &str| book.active().value(addr.parse().unwrap()).clone();
    assert_eq!(at("A1"), error(CellError::Cycle));
    assert_eq!(at("B1"), error(CellError::Cycle));
    assert_eq!(at("C1"), error(CellError::Cycle), "reads a cycle");
    assert_eq!(at("D1"), error(CellError::Cycle), "reads itself");
    assert_eq!(at("F1"), number(1.0), "unrelated formulas still work");
}

#[test]
fn a_range_containing_its_own_cell_is_a_cycle() {
    let book = calculated(&[("A1", "1"), ("A2", "=SUM(A1:A3)")]);
    assert_eq!(book.active().value("A2".parse().unwrap()), &error(CellError::Cycle));
}

#[test]
fn fixing_a_cycle_recovers() {
    let mut book = calculated(&[("A1", "=B1"), ("B1", "=A1")]);
    book.active_mut().set("B1".parse().unwrap(), Cell::parse_input("5"));
    FormulaEngine::default().recalculate(&mut book);
    assert_eq!(book.active().value("A1".parse().unwrap()), &number(5.0));
}

fn book_with_stored(formula: &str, stored: f64) -> Workbook {
    let mut sheet = Sheet::new("S");
    sheet.set("A1".parse().unwrap(), Some(Cell::formula(formula, CellValue::Number(stored))));
    sheet.set("B1".parse().unwrap(), Cell::parse_input("=A1*2"));
    Workbook::from_sheets(vec![sheet])
}

#[test]
fn unknown_functions_keep_their_formula_and_stored_value() {
    let mut book = book_with_stored("VLOOKUP(1,C1:D9,2)", 42.0);
    FormulaEngine::default().recalculate(&mut book);
    let cell = book.active().get("A1".parse().unwrap()).unwrap();
    assert_eq!(cell.value, number(42.0));
    assert!(cell.uncalculated);
    assert_eq!(cell.formula_text(), Some("VLOOKUP(1,C1:D9,2)"));
    assert_eq!(book.active().value("B1".parse().unwrap()), &number(84.0), "dependents use the stored value");
}

#[test]
fn unparsable_formulas_keep_their_stored_value() {
    let mut book = book_with_stored("{1,2}", 3.0);
    FormulaEngine::default().recalculate(&mut book);
    assert!(book.active().get("A1".parse().unwrap()).unwrap().uncalculated);
    assert_eq!(book.active().value("A1".parse().unwrap()), &number(3.0));
}

#[test]
fn a_formula_typed_with_a_bad_syntax_is_marked() {
    let book = calculated(&[("A1", "=1+")]);
    let cell = book.active().get("A1".parse().unwrap()).unwrap();
    assert!(cell.uncalculated);
}
