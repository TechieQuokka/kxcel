mod common;

use common::{calculated, fill, number};
use excel_command::Calculator;
use excel_formula_engine::FormulaEngine;
use excel_model::{CellValue, Sheet, Workbook};

fn at(book: &Workbook, addr: &str) -> CellValue {
    book.active().value(addr.parse().unwrap()).clone()
}

#[test]
fn absolute_references_read_the_same_cell() {
    let book = calculated(&[("A1", "5"), ("B1", "=$A$1*2"), ("C1", "=A$1+$A1")]);
    assert_eq!(at(&book, "B1"), number(10.0));
    assert_eq!(at(&book, "C1"), number(10.0));
}

#[test]
fn formulas_read_other_formulas_in_any_order() {
    let book = calculated(&[("A1", "=B1+1"), ("B1", "=C1+1"), ("C1", "1")]);
    assert_eq!(at(&book, "A1"), number(3.0));
}

#[test]
fn ranges_include_formula_results() {
    let book = calculated(&[("A1", "2"), ("A2", "=A1*3"), ("A3", "=SUM(A1:A2)")]);
    assert_eq!(at(&book, "A3"), number(8.0));
}

#[test]
fn reversed_ranges_work() {
    let book = calculated(&[("A1", "2"), ("A2", "3"), ("B1", "=SUM(A2:A1)")]);
    assert_eq!(at(&book, "B1"), number(5.0));
}

#[test]
fn other_sheets_can_be_referenced() {
    let mut data = Sheet::new("My Data");
    fill(&mut data, &[("A1", "7"), ("A2", "=Summary!B1+1")]);
    let mut summary = Sheet::new("Summary");
    fill(&mut summary, &[("A1", "='My Data'!A1*2"), ("B1", "=SUM('My Data'!A1:A1)"), ("C1", "='my data'!A2")]);
    let mut book = Workbook::from_sheets(vec![summary, data]);
    FormulaEngine::default().recalculate(&mut book);
    assert_eq!(at(&book, "A1"), number(14.0));
    assert_eq!(at(&book, "C1"), number(8.0));
}

#[test]
fn a_long_chain_does_not_overflow_the_stack() {
    let mut sheet = Sheet::new("S");
    fill(&mut sheet, &[("A1", "1")]);
    for row in 2..=200_000 {
        let cells = format!("=A{}+1", row - 1);
        fill(&mut sheet, &[(&format!("A{row}"), &cells)]);
    }
    let mut book = Workbook::from_sheets(vec![sheet]);
    FormulaEngine::default().recalculate(&mut book);
    assert_eq!(at(&book, "A200000"), number(200_000.0));
}

#[test]
fn recalculating_picks_up_changes() {
    let mut book = calculated(&[("A1", "1"), ("B1", "=A1+1")]);
    fill(book.active_mut(), &[("A1", "10")]);
    FormulaEngine::default().recalculate(&mut book);
    assert_eq!(at(&book, "B1"), number(11.0));
}
