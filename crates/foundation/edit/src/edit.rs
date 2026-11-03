use excel_model::{Cell, CellAddr, Sheet, SheetMeta, Workbook};

/// One cell's change, with what it held before and after.
#[derive(Debug, Clone, PartialEq)]
pub struct CellChange {
    pub sheet: usize,
    pub addr: CellAddr,
    pub before: Option<Cell>,
    pub after: Option<Cell>,
}

/// One sheet as it was and as it is now.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetChange {
    pub index: usize,
    pub before: Sheet,
    pub after: Sheet,
}

/// A reversible change to a workbook.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// Changes individual cells.
    SetCells(Vec<CellChange>),
    /// Replaces every sheet, for changes to the list of sheets itself, such as
    /// adding or deleting one.
    ReplaceSheets { before: Vec<Sheet>, after: Vec<Sheet> },
    /// Replaces some sheets in place. Used by changes that touch most of a
    /// sheet or rewrite formulas across sheets, such as sort, insert, and
    /// delete, keeping only the sheets they changed.
    ReplaceSome(Vec<SheetChange>),
    /// Changes a sheet's tables and other metadata, leaving its cells alone.
    /// Formulas never read metadata, so this needs no recalculation.
    SetMeta { sheet: usize, before: SheetMeta, after: SheetMeta },
    /// Several edits as one undo step, applied in order and reverted in reverse.
    Batch(Vec<Edit>),
}

impl Edit {
    /// Builds an edit by running `change` on a copy of the sheets. Only the
    /// sheets it changed are kept, unless it added or removed sheets, so undo
    /// history holds no copies of sheets that stayed the same.
    pub fn replacing_sheets(workbook: &Workbook, change: impl FnOnce(&mut Vec<Sheet>)) -> Self {
        let before = workbook.sheets().to_vec();
        let mut after = before.clone();
        change(&mut after);
        if after.len() != before.len() {
            return Self::ReplaceSheets { before, after };
        }
        let changes = before
            .into_iter()
            .zip(after)
            .enumerate()
            .filter(|(_, (before, after))| before != after)
            .map(|(index, (before, after))| SheetChange { index, before, after })
            .collect();
        Self::ReplaceSome(changes)
    }

    /// Roughly how much this edit holds, in cells, so undo history can stay
    /// within a budget. Metadata counts as one cell.
    pub fn size(&self) -> usize {
        match self {
            Self::SetCells(changes) => changes.len(),
            Self::ReplaceSheets { before, after } => before.iter().chain(after).map(Sheet::len).sum(),
            Self::ReplaceSome(changes) => changes.iter().map(|change| change.before.len() + change.after.len()).sum(),
            Self::SetMeta { .. } => 1,
            Self::Batch(edits) => edits.iter().map(Edit::size).sum(),
        }
    }

    /// The cell changes in this edit, in the order they apply, or `None` when it
    /// also replaces sheets, so only a full recalculation will do.
    pub fn cell_changes(&self) -> Option<Vec<&CellChange>> {
        match self {
            Self::SetCells(changes) => Some(changes.iter().collect()),
            Self::SetMeta { .. } => Some(Vec::new()),
            Self::ReplaceSheets { .. } | Self::ReplaceSome(_) => None,
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
            Self::ReplaceSome(changes) => {
                for change in changes {
                    workbook.sheets_mut()[change.index] = change.after.clone();
                }
            }
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
            Self::ReplaceSome(changes) => {
                for change in changes.iter().rev() {
                    workbook.sheets_mut()[change.index] = change.before.clone();
                }
            }
            Self::SetMeta { sheet, before, .. } => workbook.sheets_mut()[*sheet].set_meta(before.clone()),
            Self::Batch(edits) => edits.iter().rev().for_each(|edit| edit.revert(workbook)),
        }
    }
}
