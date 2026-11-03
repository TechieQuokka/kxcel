use crate::Edit;

/// Maximum number of edits kept for undo.
const LIMIT: usize = 100;

/// Most cells the undo history may hold, about 700 MB. Small edits stay up to
/// [`LIMIT`] deep, while edits that copy a large sheet, such as inserting a
/// row in a million cells, keep only the latest few. The latest edit is kept
/// whatever its size.
const CELL_BUDGET: usize = 5_000_000;

/// Undo and redo stacks.
#[derive(Debug)]
pub struct History {
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    budget: usize,
}

impl Default for History {
    fn default() -> Self {
        Self { undo: Vec::new(), redo: Vec::new(), budget: CELL_BUDGET }
    }
}

impl History {
    /// Records a new edit. This discards any edits that could have been redone.
    pub fn record(&mut self, edit: Edit) {
        self.redo.clear();
        self.undo.push(edit);
        self.trim();
    }

    /// Drops the oldest edits while there are too many or they hold too much.
    fn trim(&mut self) {
        let mut held: usize = self.undo.iter().map(Edit::size).sum();
        while self.undo.len() > 1 && (self.undo.len() > LIMIT || held > self.budget) {
            held -= self.undo.remove(0).size();
        }
    }

    /// How many edits undo can take back.
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    /// The edit undo would revert next, which is also the latest applied.
    pub fn last_undo(&self) -> Option<&Edit> {
        self.undo.last()
    }

    /// The edit redo would apply next, which is also the latest reverted.
    pub fn last_redo(&self) -> Option<&Edit> {
        self.redo.last()
    }

    pub(crate) fn pop_undo(&mut self) -> Option<Edit> {
        self.undo.pop()
    }

    pub(crate) fn pop_redo(&mut self) -> Option<Edit> {
        self.redo.pop()
    }

    pub(crate) fn push_undo(&mut self, edit: Edit) {
        self.undo.push(edit);
        self.trim();
    }

    pub(crate) fn push_redo(&mut self, edit: Edit) {
        self.redo.push(edit);
    }
}

#[cfg(test)]
mod tests {
    use excel_model::{Cell, CellAddr, Sheet, Workbook};

    use super::*;

    fn sheet_of(cells: u32) -> Sheet {
        let mut sheet = Sheet::new("Big");
        for row in 0..cells {
            sheet.set(CellAddr::new(row, 0), Cell::parse_input("1"));
        }
        sheet
    }

    #[test]
    fn only_changed_sheets_are_kept() {
        let book = Workbook::from_sheets(vec![sheet_of(1000), Sheet::new("Small")]);
        let edit = Edit::replacing_sheets(&book, |sheets| sheets[1].name = "Renamed".into());
        assert_eq!(edit.size(), 0, "the big sheet stayed the same, so no copy of it is kept");
        let Edit::ReplaceSome(changes) = &edit else { panic!("{edit:?}") };
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].index, 1);
        let added = Edit::replacing_sheets(&book, |sheets| sheets.push(Sheet::new("New")));
        assert!(matches!(added, Edit::ReplaceSheets { .. }), "adding a sheet replaces the list");
    }

    #[test]
    fn big_edits_stay_within_the_budget() {
        // The same rule as the real budget, at a thousandth of its size.
        let mut history = History { budget: CELL_BUDGET / 1000, ..History::default() };
        let book = Workbook::from_sheets(vec![sheet_of(1_000)]);
        for _ in 0..5 {
            history.record(Edit::replacing_sheets(&book, |sheets| sheets[0].insert_rows(0, 1)));
        }
        assert_eq!(history.undo_depth(), 2, "each holds two thousand cells");
        let huge = Workbook::from_sheets(vec![sheet_of(3_000)]);
        history.record(Edit::replacing_sheets(&huge, |sheets| sheets[0].insert_rows(0, 1)));
        assert_eq!(history.undo_depth(), 1, "the latest is kept even over the budget");
    }

    #[test]
    fn small_edits_keep_a_hundred() {
        let mut history = History::default();
        for _ in 0..150 {
            history.record(Edit::SetCells(Vec::new()));
        }
        assert_eq!(history.undo_depth(), 100);
    }
}
