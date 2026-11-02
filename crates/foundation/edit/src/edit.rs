use excel_model::{Cell, CellAddr, Sheet, Workbook};

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
}

impl Edit {
    /// Builds a `ReplaceSheets` edit by running `change` on a copy of the sheets.
    pub fn replacing_sheets(workbook: &Workbook, change: impl FnOnce(&mut Vec<Sheet>)) -> Self {
        let before = workbook.sheets().to_vec();
        let mut after = before.clone();
        change(&mut after);
        Self::ReplaceSheets { before, after }
    }

    pub(crate) fn apply(&self, workbook: &mut Workbook) {
        match self {
            Self::SetCells(changes) => {
                for change in changes {
                    workbook.sheets_mut()[change.sheet].set(change.addr, change.after.clone());
                }
            }
            Self::ReplaceSheets { after, .. } => workbook.replace_sheets(after.clone()),
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
        }
    }
}
