mod common;

use std::time::Instant;

use excel_command::{Calculator, Structural};
use excel_formula_engine::FormulaEngine;
use excel_model::{Cell, Sheet, Workbook};

#[test]
fn large_sheets_recalculate_quickly() {
    let mut sheet = Sheet::new("S");
    for row in 0..100_000u32 {
        sheet.set(excel_model::CellAddr::new(row, 0), Cell::parse_input(&row.to_string()));
        let formula = format!("=A{}*2+1", row + 1);
        sheet.set(excel_model::CellAddr::new(row, 1), Cell::parse_input(&formula));
    }
    let mut book = Workbook::from_sheets(vec![sheet]);
    let started = Instant::now();
    FormulaEngine::default().recalculate(&mut book);
    let elapsed = started.elapsed();
    assert_eq!(book.active().value("B100000".parse().unwrap()).as_number(), Some(199_999.0));
    println!("100k formulas: {elapsed:?}");
    assert!(elapsed.as_secs() < 5, "took {elapsed:?}");
}

#[test]
fn exposes_translation_and_adjustment_through_the_trait() {
    let engine = FormulaEngine::default();
    assert_eq!(engine.translate("A1+$B$1", 1, 1), "B2+$B$1");
    let change = Structural::DeleteRows { at: 0, count: 1 };
    assert_eq!(engine.adjust("A1+A2", "S", "S", change), "#REF!+A1");
}
