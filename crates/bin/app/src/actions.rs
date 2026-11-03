//! Applies what the user did in the grid to the document.

use std::collections::BTreeMap;

use excel_command::Calculator;
use excel_edit::{CellChange, Document, Edit};
use excel_model::{Cell, CellAddr, CellInput, CellRange, NumberFormat};
use excel_ui_grid::{parse_tsv, to_tsv};

use crate::series;

/// Finishes editing a cell. Blank text clears it. Returns whether the cell
/// changed.
pub fn commit(doc: &mut Document, calculator: &dyn Calculator, addr: CellAddr, text: &str) -> bool {
    apply(doc, calculator, BTreeMap::from([(addr, Cell::parse_input(text))]))
}

/// Clears every stored cell in a range.
pub fn clear(doc: &mut Document, calculator: &dyn Calculator, range: CellRange) {
    let changes = doc.workbook.active().iter_range(range).map(|(addr, _)| (addr, None)).collect();
    apply(doc, calculator, changes);
}

/// Clears the stored cells of `range` that lie inside one of `within`, as app
/// mode clears only input cells. One undo step.
pub fn clear_within(doc: &mut Document, calculator: &dyn Calculator, range: CellRange, within: &[CellRange]) {
    let changes = doc
        .workbook
        .active()
        .iter_range(range)
        .filter(|(addr, _)| within.iter().any(|inside| inside.contains(*addr)))
        .map(|(addr, _)| (addr, None))
        .collect();
    apply(doc, calculator, changes);
}

/// Continues `source` across `target`.
///
/// Each line along the fill direction, such as each column when filling down,
/// continues its own series if it has one. Otherwise its cells repeat, with
/// relative references in formulas adjusted. `alternate` swaps copying and
/// continuing, as Ctrl does in Excel.
pub fn fill(doc: &mut Document, calculator: &dyn Calculator, source: CellRange, target: CellRange, alternate: bool) {
    let sheet = doc.workbook.active();
    let (rows, cols) = (i64::from(source.rows()), i64::from(source.cols()));
    let vertical = target.rows() != source.rows();
    let mut changes = BTreeMap::new();

    for addr in target.iter().filter(|addr| !source.contains(*addr)) {
        let (line, position): (Vec<Option<&Cell>>, i64) = if vertical {
            let line = (source.start.row..=source.end.row).map(|row| sheet.get(CellAddr::new(row, addr.col))).collect();
            (line, i64::from(addr.row) - i64::from(source.start.row))
        } else {
            let line = (source.start.col..=source.end.col).map(|col| sheet.get(CellAddr::new(addr.row, col))).collect();
            (line, i64::from(addr.col) - i64::from(source.start.col))
        };
        // As in Excel, a single date counts up a day at a time, and Ctrl copies it.
        let single_date = matches!(line[..], [Some(cell)] if cell.format == NumberFormat::Date);
        let alternate = alternate != single_date;
        if let Some(mut cell) = series::detect(&line, alternate).and_then(|series| series.cell_at(position)) {
            // A series of dates continues as dates.
            if let Some(Some(first)) = line.first()
                && line.iter().all(|other| other.is_some_and(|other| other.format == first.format))
            {
                cell.format = first.format;
            }
            changes.insert(addr, Some(cell));
            continue;
        }

        let row_offset = i64::from(addr.row) - i64::from(source.start.row);
        let col_offset = i64::from(addr.col) - i64::from(source.start.col);
        let from = CellAddr::new(
            source.start.row + row_offset.rem_euclid(rows) as u32,
            source.start.col + col_offset.rem_euclid(cols) as u32,
        );
        let moved_rows = i64::from(addr.row) - i64::from(from.row);
        let moved_cols = i64::from(addr.col) - i64::from(from.col);
        let cell = sheet.get(from).map(|cell| moved(cell, calculator, moved_rows, moved_cols));
        changes.insert(addr, cell);
    }
    apply(doc, calculator, changes);
}

/// A copy of cells, recognized when the same text comes back from the clipboard.
#[derive(Debug, Clone, PartialEq)]
pub struct CopiedCells {
    tsv: String,
    source: CellRange,
    sheet: usize,
    cut: bool,
    cells: Vec<(CellAddr, Cell)>,
}

impl CopiedCells {
    /// Captures a range. Returns the capture and the text for the system clipboard.
    pub fn capture(doc: &Document, range: CellRange, cut: bool) -> (Self, String) {
        let sheet = doc.workbook.active();
        let tsv = to_tsv(sheet, range);
        let cells = sheet.iter_range(range).map(|(addr, cell)| (addr, cell.clone())).collect();
        let copied = Self { tsv: tsv.clone(), source: range, sheet: doc.workbook.active_index(), cut, cells };
        (copied, tsv)
    }
}

/// Pastes clipboard text at `at`. If the text is what this application copied,
/// formulas come along with adjusted references. Otherwise it is read as typed
/// values. A cut's source cells are cleared, and a cut can be pasted only once.
pub fn paste(
    doc: &mut Document,
    calculator: &dyn Calculator,
    copied: &mut Option<CopiedCells>,
    at: CellAddr,
    text: &str,
) {
    paste_into(doc, calculator, copied, at, text, |_| true);
}

/// Pastes as [`paste`] does, changing only cells `allowed` accepts, as app
/// mode pastes only into input cells. A cut's source cells stay unless allowed.
pub fn paste_into(
    doc: &mut Document,
    calculator: &dyn Calculator,
    copied: &mut Option<CopiedCells>,
    at: CellAddr,
    text: &str,
    allowed: impl Fn(CellAddr) -> bool,
) {
    let mut changes = BTreeMap::new();
    match copied.as_ref().filter(|copied| copied.tsv == text) {
        Some(copy) => {
            let moved_rows = i64::from(at.row) - i64::from(copy.source.start.row);
            let moved_cols = i64::from(at.col) - i64::from(copy.source.start.col);
            if copy.cut && copy.sheet == doc.workbook.active_index() {
                for (addr, _) in &copy.cells {
                    changes.insert(*addr, None);
                }
            }
            for addr in copy.source.iter() {
                if let Some(target) = addr.offset(moved_rows, moved_cols) {
                    changes.insert(target, None);
                }
            }
            for (addr, cell) in &copy.cells {
                if let Some(target) = addr.offset(moved_rows, moved_cols) {
                    // A cut keeps its references, because the cells have moved rather than multiplied.
                    let cell = if copy.cut { cell.clone() } else { moved(cell, calculator, moved_rows, moved_cols) };
                    changes.insert(target, Some(cell));
                }
            }
            if copy.cut {
                *copied = None;
            }
        }
        None => {
            for (row_index, row) in parse_tsv(text).iter().enumerate() {
                for (col_index, value) in row.iter().enumerate() {
                    if let Some(target) = at.offset(row_index as i64, col_index as i64) {
                        changes.insert(target, Cell::parse_input(value));
                    }
                }
            }
        }
    }
    changes.retain(|addr, _| allowed(*addr));
    apply(doc, calculator, changes);
}

/// A cell as it should be at a new position.
fn moved(cell: &Cell, calculator: &dyn Calculator, rows: i64, cols: i64) -> Cell {
    let mut cell = cell.clone();
    if let CellInput::Formula(text) = &cell.input {
        cell.input = CellInput::Formula(calculator.translate(text, rows, cols));
    }
    cell
}

fn apply(doc: &mut Document, calculator: &dyn Calculator, changes: BTreeMap<CellAddr, Option<Cell>>) -> bool {
    let index = doc.workbook.active_index();
    let sheet = doc.workbook.active();
    let changes: Vec<CellChange> = changes
        .into_iter()
        .map(|(addr, after)| CellChange { sheet: index, addr, before: sheet.get(addr).cloned(), after })
        .filter(|change| change.before != change.after)
        .collect();
    if changes.is_empty() {
        return false;
    }
    doc.apply(Edit::SetCells(changes));
    if let Some(edit) = doc.history.last_undo() {
        calculator.update(&mut doc.workbook, edit);
    }
    true
}

#[cfg(test)]
mod tests {
    use excel_features::Services;
    use excel_model::CellValue;

    use super::*;

    fn addr(text: &str) -> CellAddr {
        text.parse().unwrap()
    }

    fn range(text: &str) -> CellRange {
        text.parse().unwrap()
    }

    struct Fixture {
        doc: Document,
        services: Services,
    }

    impl Fixture {
        fn new(cells: &[(&str, &str)]) -> Self {
            let mut fixture = Self { doc: Document::default(), services: Services::default() };
            for (at, text) in cells {
                fixture.commit(at, text);
            }
            fixture
        }

        fn commit(&mut self, at: &str, text: &str) {
            commit(&mut self.doc, &self.services.calculator, addr(at), text);
        }

        fn value(&self, at: &str) -> String {
            self.doc.workbook.active().value(addr(at)).to_string()
        }

        fn input(&self, at: &str) -> String {
            self.doc.workbook.active().get(addr(at)).map(Cell::input_text).unwrap_or_default()
        }
    }

    fn fill_copy(doc: &mut Document, calculator: &dyn Calculator, source: CellRange, target: CellRange) {
        fill(doc, calculator, source, target, false);
    }

    fn values(f: &Fixture, addrs: &[&str]) -> Vec<String> {
        addrs.iter().map(|at| f.value(at)).collect()
    }

    #[test]
    fn filling_continues_number_series_down_and_up() {
        let mut f = Fixture::new(&[("A3", "1"), ("A4", "2"), ("A5", "3"), ("B3", "1"), ("B4", "2"), ("B5", "4")]);
        fill(&mut f.doc, &f.services.calculator, range("A3:B5"), range("A3:B8"), false);
        assert_eq!(values(&f, &["A6", "A7", "A8"]), ["4", "5", "6"]);
        assert_eq!(values(&f, &["B6", "B7", "B8"]), ["8", "16", "32"], "each column has its own series");
        fill(&mut f.doc, &f.services.calculator, range("A3:B5"), range("A1:B5"), false);
        assert_eq!(values(&f, &["A1", "A2", "B1", "B2"]), ["-1", "0", "0.25", "0.5"]);
    }

    #[test]
    fn filling_right_continues_a_row() {
        let mut f = Fixture::new(&[("A1", "10"), ("B1", "20")]);
        fill(&mut f.doc, &f.services.calculator, range("A1:B1"), range("A1:D1"), false);
        assert_eq!(values(&f, &["C1", "D1"]), ["30", "40"]);
    }

    #[test]
    fn a_single_number_copies_and_ctrl_counts() {
        let mut f = Fixture::new(&[("A1", "7")]);
        fill(&mut f.doc, &f.services.calculator, range("A1"), range("A1:A3"), false);
        assert_eq!(values(&f, &["A2", "A3"]), ["7", "7"]);
        fill(&mut f.doc, &f.services.calculator, range("A1"), range("A1:A3"), true);
        assert_eq!(values(&f, &["A2", "A3"]), ["8", "9"]);
    }

    #[test]
    fn dates_fill_as_dates_and_a_single_date_counts_days() {
        let mut f = Fixture::new(&[("A1", "2026-01-30"), ("B1", "2026-01-01"), ("B2", "2026-01-08")]);
        fill(&mut f.doc, &f.services.calculator, range("A1"), range("A1:A3"), false);
        assert_eq!(displays(&f, &["A2", "A3"]), ["2026-01-31", "2026-02-01"]);
        fill(&mut f.doc, &f.services.calculator, range("A1"), range("A1:A3"), true);
        assert_eq!(displays(&f, &["A2", "A3"]), ["2026-01-30", "2026-01-30"], "ctrl copies");
        fill(&mut f.doc, &f.services.calculator, range("B1:B2"), range("B1:B4"), false);
        assert_eq!(displays(&f, &["B3", "B4"]), ["2026-01-15", "2026-01-22"], "a week apart continues weekly");
    }

    fn displays(f: &Fixture, addrs: &[&str]) -> Vec<String> {
        addrs.iter().map(|at| f.doc.workbook.active().display(addr(at))).collect()
    }

    #[test]
    fn numbered_text_counts_up() {
        let mut f = Fixture::new(&[("A1", "항목1")]);
        fill(&mut f.doc, &f.services.calculator, range("A1"), range("A1:A3"), false);
        assert_eq!(values(&f, &["A2", "A3"]), ["항목2", "항목3"]);
    }

    #[test]
    fn a_series_fill_undoes_in_one_step() {
        let mut f = Fixture::new(&[("A1", "1"), ("A2", "2")]);
        fill(&mut f.doc, &f.services.calculator, range("A1:A2"), range("A1:A9"), false);
        assert_eq!(f.value("A9"), "9");
        f.doc.undo();
        assert_eq!(f.doc.workbook.active().len(), 2);
    }

    #[test]
    fn committing_recalculates() {
        let mut f = Fixture::new(&[("A1", "2"), ("B1", "=A1*3")]);
        assert_eq!(f.value("B1"), "6");
        f.commit("A1", "5");
        assert_eq!(f.value("B1"), "15");
        assert!(f.doc.dirty);
    }

    #[test]
    fn committing_blank_text_clears() {
        let mut f = Fixture::new(&[("A1", "2")]);
        f.commit("A1", "");
        assert!(f.doc.workbook.active().get(addr("A1")).is_none());
    }

    #[test]
    fn committing_the_same_content_records_nothing() {
        let mut f = Fixture::new(&[("A1", "2")]);
        f.doc.dirty = false;
        f.commit("A1", "2");
        assert!(!f.doc.dirty);
    }

    #[test]
    fn undo_recalculates_when_asked() {
        let mut f = Fixture::new(&[("A1", "2"), ("B1", "=A1*3")]);
        f.commit("A1", "5");
        f.doc.undo();
        f.services.calculator.recalculate(&mut f.doc.workbook);
        assert_eq!(f.value("B1"), "6");
    }

    #[test]
    fn clearing_removes_a_range_in_one_undo() {
        let mut f = Fixture::new(&[("A1", "1"), ("A2", "2"), ("B5", "3")]);
        clear(&mut f.doc, &f.services.calculator, range("A1:A2"));
        assert_eq!(f.doc.workbook.active().len(), 1);
        f.doc.undo();
        assert_eq!(f.doc.workbook.active().len(), 3);
    }

    #[test]
    fn filling_down_adjusts_relative_references() {
        let mut f = Fixture::new(&[
            ("A1", "1"),
            ("B1", "10"),
            ("A2", "2"),
            ("B2", "20"),
            ("A3", "3"),
            ("B3", "30"),
            ("C1", "=A1*B1+$A$1"),
        ]);
        fill_copy(&mut f.doc, &f.services.calculator, range("C1"), range("C1:C3"));
        assert_eq!(f.input("C3"), "=A3*B3+$A$1");
        assert_eq!(f.value("C2"), "41");
        assert_eq!(f.value("C3"), "91");
    }

    #[test]
    fn filling_up_and_right_adjusts_too() {
        let mut f = Fixture::new(&[("B3", "=A3+1")]);
        fill_copy(&mut f.doc, &f.services.calculator, range("B3"), range("B1:B3"));
        assert_eq!(f.input("B1"), "=A1+1");
        fill_copy(&mut f.doc, &f.services.calculator, range("B3"), range("B3:C3"));
        assert_eq!(f.input("C3"), "=B3+1");
    }

    #[test]
    fn filling_repeats_a_multi_cell_source() {
        let mut f = Fixture::new(&[("A1", "x"), ("A2", "y")]);
        fill_copy(&mut f.doc, &f.services.calculator, range("A1:A2"), range("A1:A5"));
        let values: Vec<_> = ["A3", "A4", "A5"].iter().map(|at| f.value(at)).collect();
        assert_eq!(values, ["x", "y", "x"]);
    }

    #[test]
    fn filling_from_blank_cells_clears_the_target() {
        let mut f = Fixture::new(&[("A2", "old")]);
        fill_copy(&mut f.doc, &f.services.calculator, range("A1"), range("A1:A2"));
        assert!(f.doc.workbook.active().get(addr("A2")).is_none());
    }

    #[test]
    fn filling_can_be_undone_in_one_step() {
        let mut f = Fixture::new(&[("A1", "1")]);
        fill_copy(&mut f.doc, &f.services.calculator, range("A1"), range("A1:A9"));
        assert_eq!(f.doc.workbook.active().len(), 9);
        f.doc.undo();
        assert_eq!(f.doc.workbook.active().len(), 1);
    }

    #[test]
    fn pasting_text_from_elsewhere_reads_it_as_typed() {
        let mut f = Fixture::new(&[]);
        paste(&mut f.doc, &f.services.calculator, &mut None, addr("B2"), "1\t사과\n=B2+1\t\n");
        assert_eq!(f.value("B2"), "1");
        assert_eq!(f.value("C2"), "사과");
        assert_eq!(f.value("B3"), "2");
    }

    #[test]
    fn copying_and_pasting_adjusts_formulas() {
        let mut f = Fixture::new(&[("A1", "2"), ("B1", "=A1*2"), ("A2", "3")]);
        let (mut copied, text) = {
            let (copied, text) = CopiedCells::capture(&f.doc, range("B1"), false);
            (Some(copied), text)
        };
        paste(&mut f.doc, &f.services.calculator, &mut copied, addr("B2"), &text);
        assert_eq!(f.input("B2"), "=A2*2");
        assert_eq!(f.value("B2"), "6");
        paste(&mut f.doc, &f.services.calculator, &mut copied, addr("B3"), &text);
        assert_eq!(f.input("B3"), "=A3*2");
        assert!(copied.is_some(), "a copy can be pasted again");
    }

    #[test]
    fn cutting_moves_cells_and_keeps_references() {
        let mut f = Fixture::new(&[("A1", "2"), ("B1", "=A1*2")]);
        let (copied, text) = CopiedCells::capture(&f.doc, range("B1"), true);
        let mut copied = Some(copied);
        paste(&mut f.doc, &f.services.calculator, &mut copied, addr("D4"), &text);
        assert!(f.doc.workbook.active().get(addr("B1")).is_none());
        assert_eq!(f.input("D4"), "=A1*2");
        assert_eq!(f.value("D4"), "4");
        assert!(copied.is_none());
    }

    #[test]
    fn a_cut_overlapping_its_target_keeps_the_moved_cells() {
        let mut f = Fixture::new(&[("A1", "1"), ("A2", "2")]);
        let (copied, text) = CopiedCells::capture(&f.doc, range("A1:A2"), true);
        paste(&mut f.doc, &f.services.calculator, &mut Some(copied), addr("A2"), &text);
        assert!(f.doc.workbook.active().get(addr("A1")).is_none());
        assert_eq!(f.value("A2"), "1");
        assert_eq!(f.value("A3"), "2");
    }

    #[test]
    fn different_clipboard_text_ignores_the_copy() {
        let mut f = Fixture::new(&[("A1", "=1+1")]);
        let (copied, _) = CopiedCells::capture(&f.doc, range("A1"), false);
        paste(&mut f.doc, &f.services.calculator, &mut Some(copied), addr("B1"), "other");
        assert_eq!(f.doc.workbook.active().value(addr("B1")), &CellValue::Text("other".into()));
    }

    #[test]
    fn bad_formulas_are_marked_until_fixed() {
        let mut f = Fixture::new(&[("A1", "=1+")]);
        assert!(f.doc.workbook.active().get(addr("A1")).unwrap().uncalculated);
        f.commit("A1", "=1+1");
        assert_eq!(f.value("A1"), "2");
    }
}
