#![allow(dead_code)]

use excel_command::Calculator;
use excel_formula_engine::FormulaEngine;
use excel_model::{Cell, CellValue, Sheet, Workbook};

/// Builds a one-sheet workbook from `(address, typed text)` pairs and calculates it.
pub fn calculated(cells: &[(&str, &str)]) -> Workbook {
    let mut sheet = Sheet::new("Sheet1");
    fill(&mut sheet, cells);
    let mut book = Workbook::from_sheets(vec![sheet]);
    FormulaEngine::default().recalculate(&mut book);
    book
}

pub fn fill(sheet: &mut Sheet, cells: &[(&str, &str)]) {
    for (addr, text) in cells {
        sheet.set(addr.parse().unwrap(), Cell::parse_input(text));
    }
}

/// The value of one formula, with `A1` holding 10 and `A2` holding 4.
pub fn value_of(formula: &str) -> CellValue {
    let book = calculated(&[("A1", "10"), ("A2", "4"), ("Z9", formula)]);
    book.active().value("Z9".parse().unwrap()).clone()
}

pub fn number(n: f64) -> CellValue {
    CellValue::Number(n)
}
