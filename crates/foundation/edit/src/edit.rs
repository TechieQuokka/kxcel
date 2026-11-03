use excel_model::{Cell, CellAddr, Sheet, SheetMeta, Workbook};

/// One cell's change, with what it held before and after.
#[derive(Debug, Clone, PartialEq)]
pub struct CellChange {
    pub sheet: usize,
    pub addr: CellAddr,
    pub before: Option<Cell>,
    pub after: Option<Cell>,
}

/// A reversible change to a workbook.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// Changes individual cells.
    SetCells(Vec<CellChange>),
    /// Replaces whole sheets. Used by changes that touch most of a sheet or
    /// rewrite formulas across sheets, such as sort, insert, and delete.
    ReplaceSheets { before: Vec<Sheet>, after: Vec<Sheet> },
    /// Changes a sheet's tables and other metadata, leaving its cells alone.
    /// Formulas never read metadata, so this needs no recalculation.
    SetMeta { sheet: usize, before: SheetMeta, after: SheetMeta },
    /// Several edits as one undo step, applied in order and reverted in reverse.
    Batch(Vec<Edit>),
}

impl Edit {
    /// Builds a `ReplaceSheets` edit by running `change` on a copy of the sheets.
    pub fn replacing_sheets(workbook: &Workbook, change: impl FnOnce(&mut Vec<Sheet>)) -> Self {
        let before = workbook.sheets().to_vec();
        let mut after = before.clone();
        change(&mut after);
        Self::ReplaceSheets { before, after }
    }

    /// The cell changes in this edit, in the order they apply, or `None` when it
    /// also replaces sheets, so only a full recalculation will do.
    pub fn cell_changes(&self) -> Option<Vec<&CellChange>> {
        match self {
            Self::SetCells(changes) => Some(changes.iter().collect()),
            Self::SetMeta { .. } => Some(Vec::new()),
            Self::ReplaceSheets { .. } => None,
            Self::Batch(edits) => {
                let mut all = Vec::new();
                for edit in edits {
                    all.extend(edit.cell_changes()?);
                }
                Some(all)
            }
        }
    }

    pub(crate) fn apply(&self, workbook: &mut Workbook) {
        match self {
            Self::SetCells(changes) => {
                for change in changes {
                    workbook.sheets_mut()[change.sheet].set(change.addr, change.after.clone());
                }
            }
            Self::ReplaceSheets { after, .. } => workbook.replace_sheets(after.clone()),
            Self::SetMeta { sheet, after, .. } => workbook.sheets_mut()[*sheet].set_meta(after.clone()),
            Self::Batch(edits) => edits.iter().for_each(|edit| edit.apply(workbook)),
        }
    }

    pub(crate) fn revert(&self, workbook: &mut Workbook) {
        match self {
            Self::SetCells(changes) => {
                for change in changes.iter().rev() {
                    workbook.sheets_mut()[change.sheet].set(change.addr, change.before.clone());
                }
            }
            Self::ReplaceSheets { before, .. } => workbook.replace_sheets(before.clone()),
            Self::SetMeta { sheet, before, .. } => workbook.sheets_mut()[*sheet].set_meta(before.clone()),
            Self::Batch(edits) => edits.iter().rev().for_each(|edit| edit.revert(workbook)),
        }
    }
}
