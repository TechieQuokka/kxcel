use crate::Sheet;

/// A set of sheets, one of which is active. There is always at least one sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct Workbook {
    sheets: Vec<Sheet>,
    active: usize,
}

impl Default for Workbook {
    fn default() -> Self {
        Self::new()
    }
}

impl Workbook {
    pub fn new() -> Self {
        Self { sheets: vec![Sheet::new("Sheet1")], active: 0 }
    }

    /// Builds a workbook from sheets, or a blank one if `sheets` is empty.
    pub fn from_sheets(sheets: Vec<Sheet>) -> Self {
        if sheets.is_empty() {
            return Self::new();
        }
        Self { sheets, active: 0 }
    }

    pub fn sheets(&self) -> &[Sheet] {
        &self.sheets
    }

    pub fn sheets_mut(&mut self) -> &mut [Sheet] {
        &mut self.sheets
    }

    /// Replaces every sheet, keeping the active index in bounds.
    pub fn replace_sheets(&mut self, sheets: Vec<Sheet>) {
        assert!(!sheets.is_empty(), "a workbook needs at least one sheet");
        self.sheets = sheets;
        self.active = self.active.min(self.sheets.len() - 1);
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    /// Selects the active sheet. Returns false if `index` is out of bounds.
    pub fn set_active(&mut self, index: usize) -> bool {
        let valid = index < self.sheets.len();
        if valid {
            self.active = index;
        }
        valid
    }

    pub fn active(&self) -> &Sheet {
        &self.sheets[self.active]
    }

    pub fn active_mut(&mut self) -> &mut Sheet {
        &mut self.sheets[self.active]
    }

    /// Finds a sheet by name, ignoring case as Excel does.
    pub fn sheet_index(&self, name: &str) -> Option<usize> {
        self.sheets.iter().position(|sheet| sheet.name.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_sheet_list_becomes_a_blank_workbook() {
        assert_eq!(Workbook::from_sheets(Vec::new()).sheets().len(), 1);
    }

    #[test]
    fn replacing_sheets_keeps_active_in_bounds() {
        let mut book = Workbook::from_sheets(vec![Sheet::new("A"), Sheet::new("B")]);
        assert!(book.set_active(1));
        book.replace_sheets(vec![Sheet::new("A")]);
        assert_eq!(book.active_index(), 0);
    }

    #[test]
    fn finds_sheets_ignoring_case() {
        let book = Workbook::from_sheets(vec![Sheet::new("Data")]);
        assert_eq!(book.sheet_index("data"), Some(0));
        assert_eq!(book.sheet_index("other"), None);
    }

    #[test]
    fn rejects_out_of_bounds_active_sheet() {
        let mut book = Workbook::new();
        assert!(!book.set_active(1));
        assert_eq!(book.active_index(), 0);
    }
}
