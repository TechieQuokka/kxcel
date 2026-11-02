//! Timings for recalculation. Run with
//! `cargo test --release -p excel-formula-engine --test bench -- --ignored --nocapture`.

use std::time::Instant;

use excel_command::Calculator;
use excel_edit::{CellChange, Edit};
use excel_formula_engine::FormulaEngine;
use excel_model::{Cell, CellAddr, Sheet, Workbook};

/// 100,000 rows: a value, a formula on it, and a running total, with a sum of
/// the totals at the top. Each row's formulas read the rows above.
fn workbook() -> Workbook {
    let mut sheet = Sheet::new("S");
    for row in 0..100_000u32 {
        let r = row + 1;
        sheet.set(CellAddr::new(row, 0), Cell::parse_input(&r.to_string()));
        sheet.set(CellAddr::new(row, 1), Cell::parse_input(&format!("=A{r}*2+1")));
        let total = if row == 0 { "=B1".to_owned() } else { format!("=C{}+B{r}", r - 1) };
        sheet.set(CellAddr::new(row, 2), Cell::parse_input(&total));
    }
    sheet.set(CellAddr::new(0, 4), Cell::parse_input("=SUM(B1:B100000)"));
    Workbook::from_sheets(vec![sheet])
}

fn edit(book: &mut Workbook, addr: &str, text: &str) -> Edit {
    let addr: CellAddr = addr.parse().unwrap();
    let before = book.active().get(addr).cloned();
    let after = Cell::parse_input(text);
    book.active_mut().set(addr, after.clone());
    Edit::SetCells(vec![CellChange { sheet: 0, addr, before, after }])
}

#[test]
#[ignore]
fn timings() {
    let engine = FormulaEngine::default();
    let mut book = workbook();
    let started = Instant::now();
    engine.recalculate(&mut book);
    println!("full recalculation, 200k formulas: {:?}", started.elapsed());

    for (label, addr, text) in [
        ("value at the bottom", "A100000", "7"),
        ("value at the top", "A1", "7"),
        ("value outside every formula", "Z1", "7"),
        ("formula edited", "B50000", "=A50000*3"),
    ] {
        let change = edit(&mut book, addr, text);
        let started = Instant::now();
        engine.update(&mut book, &change);
        println!("{label}: {:?}", started.elapsed());
    }
}
