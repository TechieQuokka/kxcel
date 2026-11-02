use excel_edit::{Document, Edit};
use excel_model::{CellAddr, CellRange};

use crate::{Calculator, Effect, WorkbookStore};

/// Everything a command can reach while it runs.
pub struct Context<'a> {
    pub doc: &'a mut Document,
    pub calculator: &'a dyn Calculator,
    pub store: &'a dyn WorkbookStore,
    pub cursor: CellAddr,
    pub selection: CellRange,
    /// Requests for the host, in the order they were made.
    pub effects: Vec<Effect>,
    /// A result to show the user.
    pub message: Option<String>,
}

impl<'a> Context<'a> {
    pub fn new(doc: &'a mut Document, calculator: &'a dyn Calculator, store: &'a dyn WorkbookStore) -> Self {
        Self {
            doc,
            calculator,
            store,
            cursor: CellAddr::default(),
            selection: CellRange::single(CellAddr::default()),
            effects: Vec::new(),
            message: None,
        }
    }

    /// Applies an edit, records it for undo, and brings formulas up to date.
    pub fn apply(&mut self, edit: Edit) {
        self.doc.apply(edit);
        if let Some(edit) = self.doc.history.last_undo() {
            self.calculator.update(&mut self.doc.workbook, edit);
        }
    }

    pub fn say(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
    }
}
