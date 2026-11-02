use crate::Edit;

/// Maximum number of edits kept for undo.
const LIMIT: usize = 100;

/// Undo and redo stacks.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Edit>,
    redo: Vec<Edit>,
}

impl History {
    /// Records a new edit. This discards any edits that could have been redone.
    pub fn record(&mut self, edit: Edit) {
        self.redo.clear();
        self.undo.push(edit);
        if self.undo.len() > LIMIT {
            self.undo.remove(0);
        }
    }

    pub(crate) fn pop_undo(&mut self) -> Option<Edit> {
        self.undo.pop()
    }

    pub(crate) fn pop_redo(&mut self) -> Option<Edit> {
        self.redo.pop()
    }

    pub(crate) fn push_undo(&mut self, edit: Edit) {
        self.undo.push(edit);
    }

    pub(crate) fn push_redo(&mut self, edit: Edit) {
        self.redo.push(edit);
    }
}
