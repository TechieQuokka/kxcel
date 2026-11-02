use std::collections::{BTreeMap, HashMap};

use crate::{Cell, CellAddr, CellRange, CellValue};

/// Default column width in pixels.
pub const DEFAULT_COL_WIDTH: f32 = 96.0;

/// A worksheet. Only non-empty cells are stored, so large sheets stay cheap.
#[derive(Debug, Clone, Default)]
pub struct Sheet {
    pub name: String,
    cells: BTreeMap<CellAddr, Cell>,
    col_widths: HashMap<u32, f32>,
    /// One past the right-most column ever used. Clearing cells does not lower it.
    col_bound: u32,
}

impl PartialEq for Sheet {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.cells == other.cells && self.col_widths == other.col_widths
    }
}

impl Sheet {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Self::default() }
    }

    pub fn get(&self, addr: CellAddr) -> Option<&Cell> {
        self.cells.get(&addr)
    }

    pub fn get_mut(&mut self, addr: CellAddr) -> Option<&mut Cell> {
        self.cells.get_mut(&addr)
    }

    /// The value at `addr`, or `Empty` when there is no cell.
    pub fn value(&self, addr: CellAddr) -> &CellValue {
        static EMPTY: CellValue = CellValue::Empty;
        self.cells.get(&addr).map_or(&EMPTY, |cell| &cell.value)
    }

    pub fn set(&mut self, addr: CellAddr, cell: Option<Cell>) {
        match cell {
            Some(cell) => {
                self.col_bound = self.col_bound.max(addr.col + 1);
                self.cells.insert(addr, cell)
            }
            None => self.cells.remove(&addr),
        };
    }

    /// Rows and columns to allow for when scrolling. This is cheap enough to call
    /// every frame, and the column count can exceed what `used_range` reports.
    pub fn extent(&self) -> (u32, u32) {
        let rows = self.cells.keys().next_back().map_or(0, |addr| addr.row + 1);
        (rows, self.col_bound)
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (CellAddr, &Cell)> {
        self.cells.iter().map(|(addr, cell)| (*addr, cell))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (CellAddr, &mut Cell)> {
        self.cells.iter_mut().map(|(addr, cell)| (*addr, cell))
    }

    /// Visits stored cells inside a range, without scanning its empty cells.
    pub fn iter_range(&self, range: CellRange) -> impl Iterator<Item = (CellAddr, &Cell)> {
        self.cells
            .range(range.start..=range.end)
            .filter(move |(addr, _)| range.contains(**addr))
            .map(|(addr, cell)| (*addr, cell))
    }

    /// The smallest range holding every stored cell.
    pub fn used_range(&self) -> Option<CellRange> {
        let first = self.cells.keys().next()?;
        let last = self.cells.keys().next_back()?;
        let (min_col, max_col) =
            self.cells.keys().fold((u32::MAX, 0), |(min, max), addr| (min.min(addr.col), max.max(addr.col)));
        Some(CellRange { start: CellAddr::new(first.row, min_col), end: CellAddr::new(last.row, max_col) })
    }

    pub fn col_width(&self, col: u32) -> f32 {
        self.col_widths.get(&col).copied().unwrap_or(DEFAULT_COL_WIDTH)
    }

    pub fn set_col_width(&mut self, col: u32, width: f32) {
        self.col_widths.insert(col, width);
    }

    /// Column widths that differ from the default.
    pub fn custom_col_widths(&self) -> impl Iterator<Item = (u32, f32)> + '_ {
        self.col_widths.iter().map(|(col, width)| (*col, *width))
    }

    /// Moves cells down to open `count` rows at `at`. Cells pushed off the sheet are dropped.
    pub fn insert_rows(&mut self, at: u32, count: u32) {
        self.remap_cells(|addr| if addr.row < at { Some(addr) } else { addr.offset(i64::from(count), 0) });
    }

    /// Removes `count` rows at `at` and moves later rows up.
    pub fn delete_rows(&mut self, at: u32, count: u32) {
        self.remap_cells(|addr| {
            if addr.row < at {
                Some(addr)
            } else if addr.row < at.saturating_add(count) {
                None
            } else {
                addr.offset(-i64::from(count), 0)
            }
        });
    }

    /// Moves cells right to open `count` columns at `at`.
    pub fn insert_cols(&mut self, at: u32, count: u32) {
        self.remap_cells(|addr| if addr.col < at { Some(addr) } else { addr.offset(0, i64::from(count)) });
        self.remap_widths(|col| if col < at { Some(col) } else { Some(col + count) });
    }

    /// Removes `count` columns at `at` and moves later columns left.
    pub fn delete_cols(&mut self, at: u32, count: u32) {
        self.remap_cells(|addr| {
            if addr.col < at {
                Some(addr)
            } else if addr.col < at.saturating_add(count) {
                None
            } else {
                addr.offset(0, -i64::from(count))
            }
        });
        self.remap_widths(|col| {
            if col < at {
                Some(col)
            } else if col < at.saturating_add(count) {
                None
            } else {
                Some(col - count)
            }
        });
    }

    fn remap_cells(&mut self, map: impl Fn(CellAddr) -> Option<CellAddr>) {
        self.cells = std::mem::take(&mut self.cells)
            .into_iter()
            .filter_map(|(addr, cell)| map(addr).map(|moved| (moved, cell)))
            .collect();
        self.col_bound = self.cells.keys().map(|addr| addr.col + 1).max().unwrap_or(0);
    }

    fn remap_widths(&mut self, map: impl Fn(u32) -> Option<u32>) {
        self.col_widths = std::mem::take(&mut self.col_widths)
            .into_iter()
            .filter_map(|(col, width)| map(col).map(|moved| (moved, width)))
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(text: &str) -> CellAddr {
        text.parse().unwrap()
    }

    fn sheet_with(addrs: &[&str]) -> Sheet {
        let mut sheet = Sheet::new("S");
        for (i, text) in addrs.iter().enumerate() {
            sheet.set(addr(text), Some(Cell::literal(CellValue::Number(i as f64))));
        }
        sheet
    }

    fn addrs(sheet: &Sheet) -> Vec<String> {
        sheet.iter().map(|(a, _)| a.to_string()).collect()
    }

    #[test]
    fn used_range_spans_all_cells() {
        let sheet = sheet_with(&["C1", "A3", "B2"]);
        assert_eq!(sheet.used_range().unwrap().to_string(), "A1:C3");
        assert_eq!(Sheet::new("E").used_range(), None);
    }

    #[test]
    fn extent_covers_cells_and_follows_structure_changes() {
        let mut sheet = sheet_with(&["C1", "A3"]);
        assert_eq!(sheet.extent(), (3, 3));
        sheet.delete_cols(2, 1);
        assert_eq!(sheet.extent(), (3, 1));
    }

    #[test]
    fn equality_ignores_the_column_bound() {
        let mut sheet = sheet_with(&["A1"]);
        let original = sheet.clone();
        sheet.set(addr("Z9"), Some(Cell::literal(CellValue::Number(1.0))));
        sheet.set(addr("Z9"), None);
        assert_eq!(sheet, original);
    }

    #[test]
    fn iter_range_skips_outside_columns() {
        let sheet = sheet_with(&["A1", "B1", "C1", "B2", "D2"]);
        let found: Vec<_> = sheet.iter_range("B1:C2".parse().unwrap()).map(|(a, _)| a.to_string()).collect();
        assert_eq!(found, ["B1", "C1", "B2"]);
    }

    #[test]
    fn inserting_and_deleting_rows_moves_cells() {
        let mut sheet = sheet_with(&["A1", "A2", "A3"]);
        sheet.insert_rows(1, 2);
        assert_eq!(addrs(&sheet), ["A1", "A4", "A5"]);
        sheet.delete_rows(0, 1);
        assert_eq!(addrs(&sheet), ["A3", "A4"]);
    }

    #[test]
    fn inserting_and_deleting_columns_moves_cells_and_widths() {
        let mut sheet = sheet_with(&["A1", "B1", "C1"]);
        sheet.set_col_width(1, 200.0);
        sheet.insert_cols(1, 1);
        assert_eq!(addrs(&sheet), ["A1", "C1", "D1"]);
        assert_eq!(sheet.col_width(2), 200.0);
        sheet.delete_cols(0, 2);
        assert_eq!(addrs(&sheet), ["A1", "B1"]);
        assert_eq!(sheet.col_width(0), 200.0);
    }
}
