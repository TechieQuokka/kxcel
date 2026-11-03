use std::path::PathBuf;

use excel_model::Workbook;

use crate::{Edit, History};

/// An open workbook with its undo history and file state.
#[derive(Debug, Default)]
pub struct Document {
    pub workbook: Workbook,
    pub history: History,
    /// The file this document saves to.
    pub path: Option<PathBuf>,
    /// Things in the source file that saving would lose, such as formatting.
    /// Saving over the source file needs confirmation while this is not empty.
    pub loss_warnings: Vec<String>,
    /// True when there are changes that have not been saved.
    pub dirty: bool,
}

impl Document {
    pub fn new(workbook: Workbook, path: Option<PathBuf>, loss_warnings: Vec<String>) -> Self {
        Self { workbook, history: History::default(), path, loss_warnings, dirty: false }
    }

    /// Applies an edit and records it for undo. Values entered just past a
    /// table's edge grow the table, in the same undo step.
    pub fn apply(&mut self, edit: Edit) {
        let edit = crate::with_table_growth(self.workbook.sheets(), edit);
        edit.apply(&mut self.workbook);
        self.history.record(edit);
        self.dirty = true;
    }

    /// Reverts the latest edit. Returns false if there was nothing to undo.
    pub fn undo(&mut self) -> bool {
        let Some(edit) = self.history.pop_undo() else {
            return false;
        };
        edit.revert(&mut self.workbook);
        self.history.push_redo(edit);
        self.dirty = true;
        true
    }

    /// Reapplies the latest undone edit. Returns false if there was nothing to redo.
    pub fn redo(&mut self) -> bool {
        let Some(edit) = self.history.pop_redo() else {
            return false;
        };
        edit.apply(&mut self.workbook);
        self.history.push_undo(edit);
        self.dirty = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use excel_model::{Cell, CellAddr, CellValue, Sheet};

    use super::*;
    use crate::CellChange;

    const A1: CellAddr = CellAddr::new(0, 0);

    fn number(n: f64) -> Option<Cell> {
        Some(Cell::literal(CellValue::Number(n)))
    }

    fn set_a1(doc: &mut Document, n: f64) {
        let before = doc.workbook.active().get(A1).cloned();
        doc.apply(Edit::SetCells(vec![CellChange { sheet: 0, addr: A1, before, after: number(n) }]));
    }

    fn a1(doc: &Document) -> &CellValue {
        doc.workbook.active().value(A1)
    }

    #[test]
    fn undo_and_redo_restore_cells() {
        let mut doc = Document::default();
        set_a1(&mut doc, 1.0);
        set_a1(&mut doc, 2.0);
        assert!(doc.undo());
        assert_eq!(a1(&doc), &CellValue::Number(1.0));
        assert!(doc.undo());
        assert_eq!(a1(&doc), &CellValue::Empty);
        assert!(!doc.undo());
        assert!(doc.redo());
        assert!(doc.redo());
        assert_eq!(a1(&doc), &CellValue::Number(2.0));
        assert!(!doc.redo());
    }

    #[test]
    fn a_new_edit_clears_redo() {
        let mut doc = Document::default();
        set_a1(&mut doc, 1.0);
        doc.undo();
        set_a1(&mut doc, 2.0);
        assert!(!doc.redo());
    }

    #[test]
    fn replacing_sheets_can_be_undone() {
        let mut doc = Document::default();
        let edit = Edit::replacing_sheets(&doc.workbook, |sheets| sheets.push(Sheet::new("Sheet2")));
        doc.apply(edit);
        assert_eq!(doc.workbook.sheets().len(), 2);
        doc.undo();
        assert_eq!(doc.workbook.sheets().len(), 1);
    }

    #[test]
    fn edits_mark_the_document_dirty() {
        let mut doc = Document::default();
        assert!(!doc.dirty);
        set_a1(&mut doc, 1.0);
        assert!(doc.dirty);
    }

    #[test]
    fn history_is_limited() {
        let mut doc = Document::default();
        for i in 0..150 {
            set_a1(&mut doc, f64::from(i));
        }
        let mut undone = 0;
        while doc.undo() {
            undone += 1;
        }
        assert_eq!(undone, 100);
    }
}
