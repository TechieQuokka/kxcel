//! Incremental recalculation must always give what recalculating everything gives.

use excel_command::Calculator;
use excel_edit::{CellChange, Document, Edit};
use excel_formula_engine::FormulaEngine;
use excel_model::{Cell, CellAddr, Sheet, Workbook};

/// A small deterministic random sequence, so failures can be replayed.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

const ROWS: u32 = 8;
const COLS: u32 = 5;

fn name(rng: &mut Random) -> String {
    let col = (b'A' + rng.below(u64::from(COLS)) as u8) as char;
    format!("{col}{}", rng.below(u64::from(ROWS)) + 1)
}

/// Typed text for a random cell: numbers, blanks, and formulas reading other
/// cells, ranges, the other sheet, unknown functions, themselves, and dates.
fn random_input(rng: &mut Random) -> Option<String> {
    let (a, b) = (name(rng), name(rng));
    Some(match rng.below(19) {
        16 => format!("=IFERROR(VLOOKUP({a}, {a}:{b}, 1, FALSE), \"-\")"),
        17 => format!("=DATE(2026, 1, {a})"),
        18 => format!("=IF(ISNUMBER({a}), TEXT({a}, \"0.0\"), LEN({b}))"),
        14 => format!("=SUMIF({a}:{b}, \">50\")+COUNTIFS({a}:{b}, \"<>\")"),
        15 => format!("=ROUND({a}/7, 2)"),
        12 => format!("2026-{:02}-{:02}", 1 + rng.below(12), 1 + rng.below(28)),
        13 => format!("={a}+7"),
        0 => return None,
        1..=3 => rng.below(100).to_string(),
        4 => format!("={a}+{b}"),
        5 => format!("=SUM({a}:{b})"),
        6 => format!("=IF({a}>50, {b}, {a}*2)"),
        7 => format!("=Other!{a}*10"),
        8 => format!("=AVERAGE({a}:{b})+COUNT(Other!{a}:{b})"),
        9 => format!("=NOSUCH({a})"),
        10 => format!("=1/({a}-{b})"),
        _ => format!("=MAX({a}, {b}, 3)"),
    })
}

fn random_workbook(rng: &mut Random) -> Workbook {
    let mut sheets = Vec::new();
    for title in ["Main", "Other"] {
        let mut sheet = Sheet::new(title);
        for row in 0..ROWS {
            for col in 0..COLS {
                if let Some(text) = random_input(rng) {
                    sheet.set(CellAddr::new(row, col), Cell::parse_input(&text));
                }
            }
        }
        sheets.push(sheet);
    }
    Workbook::from_sheets(sheets)
}

/// Recalculates a copy from scratch with a fresh engine.
fn from_scratch(workbook: &Workbook) -> Workbook {
    let mut copy = workbook.clone();
    FormulaEngine::default().recalculate(&mut copy);
    copy
}

#[test]
fn matches_full_recalculation_after_every_edit_undo_and_redo() {
    for seed in 1..=60 {
        let mut rng = Random(seed);
        let engine = FormulaEngine::default();
        let mut doc = Document::new(random_workbook(&mut rng), None, Vec::new());
        engine.recalculate(&mut doc.workbook);

        for step in 0..100 {
            match rng.below(10) {
                0 | 1 => {
                    if doc.undo() {
                        engine.update(&mut doc.workbook, doc.history.last_redo().unwrap());
                    }
                }
                2 => {
                    if doc.redo() {
                        engine.update(&mut doc.workbook, doc.history.last_undo().unwrap());
                    }
                }
                _ => {
                    let count = 1 + rng.below(3);
                    let changes = (0..count)
                        .map(|_| {
                            let sheet = rng.below(2) as usize;
                            let addr: CellAddr = name(&mut rng).parse().unwrap();
                            let before = doc.workbook.sheets()[sheet].get(addr).cloned();
                            let after = random_input(&mut rng).and_then(|text| Cell::parse_input(&text));
                            CellChange { sheet, addr, before, after }
                        })
                        .collect();
                    doc.apply(Edit::SetCells(changes));
                    engine.update(&mut doc.workbook, doc.history.last_undo().unwrap());
                }
            }
            assert_eq!(doc.workbook, from_scratch(&doc.workbook), "seed {seed}, step {step}");
        }
    }
}

#[test]
fn another_workbook_or_whole_sheet_changes_are_calculated_from_scratch() {
    let engine = FormulaEngine::default();
    let mut first = Workbook::from_sheets(vec![Sheet::new("S")]);
    engine.recalculate(&mut first);

    let mut second = Workbook::from_sheets(vec![Sheet::new("S")]);
    second.active_mut().set("A1".parse().unwrap(), Cell::parse_input("2"));
    second.active_mut().set("B1".parse().unwrap(), Cell::parse_input("=A1*3"));
    let change = CellChange { sheet: 0, addr: "C1".parse().unwrap(), before: None, after: None };
    engine.update(&mut second, &Edit::SetCells(vec![change]));
    assert_eq!(
        second.active().value("B1".parse().unwrap()).as_number(),
        Some(6.0),
        "a graph for another workbook is not used"
    );

    let mut sheets = second.sheets().to_vec();
    sheets[0].set("A1".parse().unwrap(), Cell::parse_input("5"));
    let edit = Edit::ReplaceSheets { before: second.sheets().to_vec(), after: sheets.clone() };
    second.replace_sheets(sheets);
    engine.update(&mut second, &edit);
    assert_eq!(second.active().value("B1".parse().unwrap()).as_number(), Some(15.0));
}
