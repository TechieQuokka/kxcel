//! Tables grow when values are entered just past their edge, as in Excel.

use std::collections::BTreeSet;

use excel_model::{CellAddr, CellRange, MAX_COLS, MAX_ROWS, Sheet, SheetMeta};

use crate::{CellChange, Edit};

/// Adds table growth to an edit. A value set in the row just below a table,
/// within its columns, adds that row, and a value set in the header row just
/// right of it adds a column. Several such rows or columns in one edit all join.
/// Returns the edit unchanged when no table grows, or else a batch of it and
/// the table change, so one undo takes back both.
pub fn with_table_growth(sheets: &[Sheet], edit: Edit) -> Edit {
    let Edit::SetCells(changes) = &edit else {
        return edit;
    };
    let mut growth = Vec::new();
    for (index, sheet) in sheets.iter().enumerate().filter(|(_, sheet)| !sheet.tables().is_empty()) {
        let filled: BTreeSet<CellAddr> =
            changes.iter().filter(|change| change.sheet == index && fills(change)).map(|change| change.addr).collect();
        if filled.is_empty() {
            continue;
        }
        let before = sheet.meta().clone();
        let mut after = before.clone();
        for position in 0..after.tables.len() {
            let grown = grow(&after, position, &filled);
            after.tables[position].range = grown;
        }
        if after != before {
            growth.push(Edit::SetMeta { sheet: index, before, after });
        }
    }
    if growth.is_empty() {
        return edit;
    }
    growth.insert(0, edit);
    Edit::Batch(growth)
}

/// The edit that adds an empty row to the table whose last row holds `addr`,
/// as `Enter` does after an edit there. `None` when `addr` is not in a
/// table's last row, or the new row would leave the sheet or meet another table.
pub fn add_table_row(sheet: &Sheet, index: usize, addr: CellAddr) -> Option<Edit> {
    let position = sheet.tables().iter().position(|table| table.range.contains(addr))?;
    let range = sheet.tables()[position].range;
    let row = range.end.row.checked_add(1).filter(|row| *row < MAX_ROWS && addr.row == range.end.row)?;
    let line = CellRange::new(CellAddr::new(row, range.start.col), CellAddr::new(row, range.end.col));
    if sheet.tables().iter().any(|table| table.range.overlaps(line)) {
        return None;
    }
    let before = sheet.meta().clone();
    let mut after = before.clone();
    after.tables[position].range.end.row = row;
    Some(Edit::SetMeta { sheet: index, before, after })
}

fn fills(change: &CellChange) -> bool {
    change.after.as_ref().is_some_and(|cell| cell.formula_text().is_some() || !cell.value.is_empty())
}

/// The range of table `position` once it takes in the filled cells next to it.
fn grow(meta: &SheetMeta, position: usize, filled: &BTreeSet<CellAddr>) -> CellRange {
    let mut range = meta.tables[position].range;
    let free = |extra: CellRange| {
        meta.tables.iter().enumerate().all(|(other, table)| other == position || !table.range.overlaps(extra))
    };
    let header = range.start.row;
    while range.end.col + 1 < MAX_COLS && filled.contains(&CellAddr::new(header, range.end.col + 1)) {
        let column =
            CellRange::new(CellAddr::new(header, range.end.col + 1), CellAddr::new(range.end.row, range.end.col + 1));
        if !free(column) {
            break;
        }
        range.end.col += 1;
    }
    loop {
        let row = range.end.row + 1;
        if row >= MAX_ROWS {
            break;
        }
        let line = CellRange::new(CellAddr::new(row, range.start.col), CellAddr::new(row, range.end.col));
        if !filled.range(line.start..=line.end).any(|addr| line.contains(*addr)) || !free(line) {
            break;
        }
        range.end.row = row;
    }
    range
}

#[cfg(test)]
mod tests {
    use excel_model::{Cell, Table};

    use super::*;
    use crate::Document;

    fn addr(text: &str) -> CellAddr {
        text.parse().unwrap()
    }

    fn doc_with_table(range: &str) -> Document {
        let mut doc = Document::default();
        let sheet = doc.workbook.active_mut();
        sheet.set_meta(SheetMeta { tables: vec![Table::new("T", range.parse().unwrap())], ..Default::default() });
        doc
    }

    fn set(doc: &mut Document, cells: &[(&str, &str)]) {
        let changes = cells
            .iter()
            .map(|(at, text)| CellChange {
                sheet: 0,
                addr: addr(at),
                before: doc.workbook.active().get(addr(at)).cloned(),
                after: Cell::parse_input(text),
            })
            .collect();
        doc.apply(Edit::SetCells(changes));
    }

    fn range(doc: &Document) -> String {
        doc.workbook.active().tables()[0].range.to_string()
    }

    #[test]
    fn a_value_below_adds_a_row() {
        let mut doc = doc_with_table("A1:B3");
        set(&mut doc, &[("B4", "5")]);
        assert_eq!(range(&doc), "A1:B4");
    }

    #[test]
    fn a_header_to_the_right_adds_a_column() {
        let mut doc = doc_with_table("A1:B3");
        set(&mut doc, &[("C1", "가격")]);
        assert_eq!(range(&doc), "A1:C3");
        set(&mut doc, &[("D2", "x")]);
        assert_eq!(range(&doc), "A1:C3", "only the header row adds columns");
    }

    #[test]
    fn diagonal_far_and_outside_cells_do_not_grow_it() {
        let mut doc = doc_with_table("A1:B3");
        set(&mut doc, &[("C4", "1"), ("A6", "1"), ("C2", "1")]);
        assert_eq!(range(&doc), "A1:B3");
        set(&mut doc, &[("A4", "")]);
        assert_eq!(range(&doc), "A1:B3", "clearing a cell is not a value");
    }

    #[test]
    fn several_rows_in_one_edit_all_join() {
        let mut doc = doc_with_table("A1:B3");
        set(&mut doc, &[("A4", "1"), ("A5", "2"), ("A7", "3")]);
        assert_eq!(range(&doc), "A1:B5");
    }

    #[test]
    fn growth_is_one_undo_step() {
        let mut doc = doc_with_table("A1:B3");
        set(&mut doc, &[("A4", "1")]);
        assert!(doc.undo());
        assert_eq!(range(&doc), "A1:B3");
        assert!(doc.workbook.active().get(addr("A4")).is_none());
        assert!(doc.redo());
        assert_eq!(range(&doc), "A1:B4");
    }

    #[test]
    fn enter_in_the_last_row_adds_one_joined_to_the_edit() {
        let mut doc = doc_with_table("A1:B3");
        assert!(add_table_row(doc.workbook.active(), 0, addr("B2")).is_none(), "not the last row");
        assert!(add_table_row(doc.workbook.active(), 0, addr("C3")).is_none(), "outside the table");
        set(&mut doc, &[("B3", "7")]);
        let edit = add_table_row(doc.workbook.active(), 0, addr("B3")).unwrap();
        doc.apply_joined(edit);
        assert_eq!(range(&doc), "A1:B4");
        assert!(doc.undo());
        assert_eq!(range(&doc), "A1:B3");
        assert!(doc.workbook.active().get(addr("B3")).is_none(), "one undo takes back both");
        assert!(doc.redo());
        assert_eq!(range(&doc), "A1:B4");
    }

    #[test]
    fn a_table_does_not_grow_into_another() {
        let mut doc = doc_with_table("A1:B3");
        let mut meta = doc.workbook.active().meta().clone();
        meta.tables.push(Table::new("U", "A4:B6".parse().unwrap()));
        doc.workbook.active_mut().set_meta(meta);
        set(&mut doc, &[("A4", "1")]);
        assert_eq!(range(&doc), "A1:B3");
    }
}
