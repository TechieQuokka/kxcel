use std::collections::{BTreeMap, HashMap};

use crate::table::Shift;
use crate::{Cell, CellAddr, CellRange, CellValue, MAX_COLS, RowSet, SheetMeta, Table};

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
    meta: SheetMeta,
}

impl PartialEq for Sheet {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.cells == other.cells
            && self.col_widths == other.col_widths
            && self.meta == other.meta
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

    /// A cell's value as it is shown, following its format. Empty for no cell.
    pub fn display(&self, addr: CellAddr) -> String {
        self.get(addr).map(Cell::display).unwrap_or_default()
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

    /// True when `addr` holds something: a value, or a formula even if it
    /// gives nothing.
    pub fn is_filled(&self, addr: CellAddr) -> bool {
        self.get(addr).is_some_and(|cell| cell.formula_text().is_some() || !cell.value.is_empty())
    }

    /// True when `addr` is filled and its row is not hidden.
    pub fn is_shown_filled(&self, addr: CellAddr) -> bool {
        self.is_filled(addr) && !self.is_row_hidden(addr.row)
    }

    /// Rows hidden by table filters.
    pub fn hidden_rows(&self) -> &RowSet {
        &self.meta.hidden_rows
    }

    /// Whether an app lets its users change this cell.
    pub fn is_input(&self, addr: CellAddr) -> bool {
        self.meta.input_cells.contains(addr)
    }

    pub fn is_row_hidden(&self, row: u32) -> bool {
        self.meta.hidden_rows.contains(row)
    }

    /// `row` if it shows, else the nearest showing row past it, down or up.
    pub fn shown_row(&self, row: u32, down: bool) -> Option<u32> {
        self.meta.hidden_rows.skip(row, down)
    }

    /// The rows that `tables`' filters hide, judged by the cells as they are now.
    pub fn rows_hidden_by(&self, tables: &[Table]) -> RowSet {
        let mut hidden = RowSet::default();
        for table in tables.iter().filter(|table| !table.filters.is_empty()) {
            for row in table.data_rows() {
                let passes = table.filters.iter().all(|filter| {
                    let addr = CellAddr::new(row, filter.column);
                    filter.rule.passes(self.value(addr), &self.display(addr))
                });
                if !passes {
                    hidden.insert(row);
                }
            }
        }
        hidden
    }

    /// The nearest filled cell after `from` in one direction, given as a step
    /// of -1, 0, or 1 in rows and columns, one of them zero. Skips gaps using the
    /// ordered cell map, so a sparse sheet is cheap to search. Going up or down,
    /// hidden rows count as empty.
    pub fn next_filled(&self, from: CellAddr, rows: i8, cols: i8) -> Option<CellAddr> {
        let filled = |addr: &&CellAddr| self.is_filled(**addr);
        match (rows, cols) {
            (0, 1) => {
                let start = from.offset(0, 1)?;
                let end = CellAddr::new(from.row, MAX_COLS - 1);
                self.cells.range(start..=end).map(|(addr, _)| addr).find(filled).copied()
            }
            (0, -1) => {
                let start = CellAddr::new(from.row, 0);
                self.cells.range(start..from).map(|(addr, _)| addr).rev().find(filled).copied()
            }
            // Down and up visit only rows that hold cells, one lookup each, so
            // wide sheets do not cost a scan of every cell.
            (1, 0) => {
                let mut row = from.row;
                loop {
                    row = self.cells.range(CellAddr::new(row.checked_add(1)?, 0)..).next()?.0.row;
                    if self.is_row_hidden(row) {
                        // Past the whole hidden run at once.
                        row = self.shown_row(row, true)? - 1;
                    } else if self.is_filled(CellAddr::new(row, from.col)) {
                        return Some(CellAddr::new(row, from.col));
                    }
                }
            }
            (-1, 0) => {
                let mut row = from.row;
                loop {
                    row = self.cells.range(..CellAddr::new(row, 0)).next_back()?.0.row;
                    if self.is_row_hidden(row) {
                        row = self.shown_row(row, false)? + 1;
                    } else if self.is_filled(CellAddr::new(row, from.col)) {
                        return Some(CellAddr::new(row, from.col));
                    }
                }
            }
            _ => None,
        }
    }

    /// The smallest range inside `range` that holds all its filled cells, so
    /// empty rows and columns at its edges are left out. `None` when nothing
    /// in it is filled.
    pub fn filled_bounds(&self, range: CellRange) -> Option<CellRange> {
        let mut bounds: Option<CellRange> = None;
        for (at, _) in self.iter_range(range).filter(|(at, _)| self.is_filled(*at)) {
            bounds = Some(match bounds {
                None => CellRange::single(at),
                Some(b) => CellRange::new(
                    CellAddr::new(b.start.row.min(at.row), b.start.col.min(at.col)),
                    CellAddr::new(b.end.row.max(at.row), b.end.col.max(at.col)),
                ),
            });
        }
        bounds
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

    /// Tables, and anything else held besides cells and column widths.
    pub fn meta(&self) -> &SheetMeta {
        &self.meta
    }

    pub fn set_meta(&mut self, meta: SheetMeta) {
        self.meta = meta;
    }

    pub fn tables(&self) -> &[Table] {
        &self.meta.tables
    }

    /// The table holding `addr`.
    pub fn table_at(&self, addr: CellAddr) -> Option<&Table> {
        self.meta.tables.iter().find(|table| table.range.contains(addr))
    }

    /// The table with this name, ignoring case.
    pub fn table_by_name(&self, name: &str) -> Option<&Table> {
        self.meta.tables.iter().find(|table| table.name.eq_ignore_ascii_case(name))
    }

    /// The column of `table` whose header is `name`, ignoring case.
    pub fn table_column(&self, table: &Table, name: &str) -> Option<u32> {
        let name = name.trim();
        (table.range.start.col..=table.range.end.col)
            .find(|col| self.display(CellAddr::new(table.header_row(), *col)).trim().eq_ignore_ascii_case(name))
    }

    /// Moves cells down to open `count` rows at `at`. Cells pushed off the sheet are dropped.
    pub fn insert_rows(&mut self, at: u32, count: u32) {
        self.remap_cells(|addr| if addr.row < at { Some(addr) } else { addr.offset(i64::from(count), 0) });
        self.meta.shift(Shift { rows: true, insert: true, at, count });
    }

    /// Removes `count` rows at `at` and moves later rows up.
    pub fn delete_rows(&mut self, at: u32, count: u32) {
        self.meta.shift(Shift { rows: true, insert: false, at, count });
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
        self.meta.shift(Shift { rows: false, insert: true, at, count });
    }

    /// Removes `count` columns at `at` and moves later columns left.
    pub fn delete_cols(&mut self, at: u32, count: u32) {
        self.meta.shift(Shift { rows: false, insert: false, at, count });
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

#[cfg(test)]
mod table_tests {
    use super::*;

    fn sheet_with_table(range: &str) -> Sheet {
        let mut sheet = Sheet::new("S");
        sheet.set_meta(SheetMeta { tables: vec![Table::new("T", range.parse().unwrap())], ..Default::default() });
        sheet
    }

    fn table_range(sheet: &Sheet) -> Option<String> {
        sheet.tables().first().map(|table| table.range.to_string())
    }

    #[test]
    fn rows_inserted_above_or_inside_move_or_grow_tables() {
        let mut sheet = sheet_with_table("B2:C5");
        sheet.insert_rows(0, 2);
        assert_eq!(table_range(&sheet).unwrap(), "B4:C7");
        sheet.insert_rows(5, 1);
        assert_eq!(table_range(&sheet).unwrap(), "B4:C8");
        sheet.insert_rows(20, 1);
        assert_eq!(table_range(&sheet).unwrap(), "B4:C8");
    }

    #[test]
    fn deleting_rows_shrinks_tables_and_the_header_row_removes_them() {
        let mut sheet = sheet_with_table("A2:B6");
        sheet.delete_rows(0, 1);
        assert_eq!(table_range(&sheet).unwrap(), "A1:B5");
        sheet.delete_rows(2, 2);
        assert_eq!(table_range(&sheet).unwrap(), "A1:B3");
        sheet.delete_rows(2, 10);
        assert_eq!(table_range(&sheet).unwrap(), "A1:B2");
        sheet.delete_rows(0, 1);
        assert_eq!(table_range(&sheet), None);
    }

    #[test]
    fn columns_move_tables_and_deleting_them_all_removes_one() {
        let mut sheet = sheet_with_table("C1:E3");
        sheet.insert_cols(3, 1);
        assert_eq!(table_range(&sheet).unwrap(), "C1:F3");
        sheet.delete_cols(1, 2);
        assert_eq!(table_range(&sheet).unwrap(), "B1:D3");
        sheet.delete_cols(0, 10);
        assert_eq!(table_range(&sheet), None);
    }

    #[test]
    fn tables_are_found_by_cell_name_and_header() {
        let mut sheet = sheet_with_table("A1:B3");
        sheet.set("B1".parse().unwrap(), Cell::parse_input("가격"));
        let table = sheet.table_at("B3".parse().unwrap()).unwrap().clone();
        assert!(sheet.table_at("C1".parse().unwrap()).is_none());
        assert_eq!(sheet.table_by_name("t").unwrap().name, "T");
        assert_eq!(sheet.table_column(&table, " 가격 "), Some(1));
        assert_eq!(sheet.table_column(&table, "nope"), None);
    }

    #[test]
    fn filled_bounds_trim_empty_edges() {
        let mut sheet = Sheet::new("S");
        for at in ["B2", "D2", "C4"] {
            sheet.set(at.parse().unwrap(), Cell::parse_input("1"));
        }
        let bounds = |range: &str| sheet.filled_bounds(range.parse().unwrap()).map(|r| r.to_string());
        assert_eq!(bounds("A1:F9").as_deref(), Some("B2:D4"));
        assert_eq!(bounds("C1:F9").as_deref(), Some("C2:D4"), "only what is inside counts");
        assert_eq!(bounds("E5:F9"), None);
    }

    #[test]
    fn equality_compares_tables() {
        let sheet = sheet_with_table("A1:B3");
        assert_ne!(sheet, Sheet::new("S"));
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;

    #[test]
    fn input_cells_move_with_inserted_and_deleted_rows_and_columns() {
        let mut sheet = Sheet::new("S");
        let mut meta = sheet.meta().clone();
        meta.input_cells.add("B2:C3".parse().unwrap());
        sheet.set_meta(meta);
        sheet.insert_rows(0, 1);
        sheet.insert_cols(2, 1);
        let ranges: Vec<String> = sheet.meta().input_cells.ranges().iter().map(|r| r.to_string()).collect();
        assert_eq!(ranges, ["B3:D4"]);
        sheet.delete_rows(2, 1);
        sheet.delete_cols(0, 2);
        let ranges: Vec<String> = sheet.meta().input_cells.ranges().iter().map(|r| r.to_string()).collect();
        assert_eq!(ranges, ["A3:B3"]);
        assert!(sheet.is_input("B3".parse().unwrap()));
        sheet.delete_rows(0, 5);
        assert!(sheet.meta().input_cells.is_empty());
    }
}
