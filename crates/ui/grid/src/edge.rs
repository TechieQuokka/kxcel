//! Where `Ctrl+Arrow` lands: the edge of the data in one direction.

use excel_model::{CellAddr, MAX_COLS, MAX_ROWS, Sheet};

/// A direction to jump in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn step(self) -> (i8, i8) {
        match self {
            Self::Up => (-1, 0),
            Self::Down => (1, 0),
            Self::Left => (0, -1),
            Self::Right => (0, 1),
        }
    }

    /// The last cell of the sheet in this direction, in `from`'s row or column.
    fn sheet_edge(self, from: CellAddr) -> CellAddr {
        match self {
            Self::Up => CellAddr::new(0, from.col),
            Self::Down => CellAddr::new(MAX_ROWS - 1, from.col),
            Self::Left => CellAddr::new(from.row, 0),
            Self::Right => CellAddr::new(from.row, MAX_COLS - 1),
        }
    }
}

/// Where `Ctrl+Arrow` moves from `from`, as Excel does:
///
/// - From a filled cell whose neighbor is filled, to the last filled cell of
///   that run.
/// - Otherwise, to the next filled cell in that direction.
/// - With nothing further, to the sheet edge.
pub fn data_edge(sheet: &Sheet, from: CellAddr, direction: Direction) -> CellAddr {
    let (rows, cols) = direction.step();
    // The next cell this way, passing over hidden rows.
    let step = |addr: CellAddr| {
        let next = addr.offset(i64::from(rows), i64::from(cols))?;
        let row = if rows == 0 { next.row } else { sheet.shown_row(next.row, rows > 0)? };
        Some(CellAddr::new(row, next.col))
    };
    let Some(next) = step(from) else {
        return from;
    };
    if sheet.is_filled(from) && sheet.is_shown_filled(next) {
        let mut last = next;
        while let Some(further) = step(last).filter(|addr| sheet.is_shown_filled(*addr)) {
            last = further;
        }
        return last;
    }
    if let Some(found) = sheet.next_filled(from, rows, cols) {
        return found;
    }
    let edge = direction.sheet_edge(from);
    match sheet.shown_row(edge.row, rows < 0) {
        Some(row) => CellAddr::new(row, edge.col),
        None => from,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_model::{Cell, CellValue};

    fn addr(text: &str) -> CellAddr {
        text.parse().unwrap()
    }

    fn sheet_with(addrs: &[&str]) -> Sheet {
        let mut sheet = Sheet::new("S");
        for text in addrs {
            sheet.set(addr(text), Some(Cell::literal(CellValue::Number(1.0))));
        }
        sheet
    }

    fn jump(sheet: &Sheet, from: &str, direction: Direction) -> String {
        data_edge(sheet, addr(from), direction).to_string()
    }

    #[test]
    fn runs_end_at_their_last_filled_cell() {
        let sheet = sheet_with(&["A1", "A2", "A3", "A6", "A7"]);
        assert_eq!(jump(&sheet, "A1", Direction::Down), "A3");
        assert_eq!(jump(&sheet, "A3", Direction::Down), "A6", "from a run's end to the next run");
        assert_eq!(jump(&sheet, "A6", Direction::Down), "A7");
        assert_eq!(jump(&sheet, "A7", Direction::Down), "A1048576", "nothing further");
        assert_eq!(jump(&sheet, "A7", Direction::Up), "A6");
        assert_eq!(jump(&sheet, "A6", Direction::Up), "A3");
        assert_eq!(jump(&sheet, "A2", Direction::Up), "A1");
    }

    #[test]
    fn from_an_empty_cell_to_the_next_filled_one() {
        let sheet = sheet_with(&["C5", "H5"]);
        assert_eq!(jump(&sheet, "A5", Direction::Right), "C5");
        assert_eq!(jump(&sheet, "C5", Direction::Right), "H5");
        assert_eq!(jump(&sheet, "H5", Direction::Right), "XFD5");
        assert_eq!(jump(&sheet, "Z5", Direction::Left), "H5");
        assert_eq!(jump(&sheet, "C5", Direction::Left), "A5");
        assert_eq!(jump(&sheet, "C1", Direction::Down), "C5");
    }

    #[test]
    fn a_single_cell_and_an_empty_sheet() {
        let sheet = sheet_with(&["B2"]);
        assert_eq!(jump(&sheet, "B2", Direction::Down), "B1048576");
        assert_eq!(jump(&sheet, "B2", Direction::Up), "B1");
        let empty = Sheet::new("E");
        assert_eq!(jump(&empty, "D4", Direction::Up), "D1");
        assert_eq!(jump(&empty, "D4", Direction::Down), "D1048576");
        assert_eq!(jump(&empty, "D4", Direction::Left), "A4");
        assert_eq!(jump(&empty, "D4", Direction::Right), "XFD4");
    }

    #[test]
    fn at_the_sheet_edge_it_stays() {
        let sheet = sheet_with(&["A1"]);
        assert_eq!(jump(&sheet, "A1", Direction::Up), "A1");
        assert_eq!(jump(&sheet, "A1", Direction::Left), "A1");
        assert_eq!(jump(&sheet, "XFD1048576", Direction::Down), "XFD1048576");
        assert_eq!(jump(&sheet, "XFD1048576", Direction::Right), "XFD1048576");
    }

    #[test]
    fn other_columns_do_not_count() {
        let sheet = sheet_with(&["A1", "B2", "C3", "A9"]);
        assert_eq!(jump(&sheet, "A1", Direction::Down), "A9");
        assert_eq!(jump(&sheet, "A9", Direction::Up), "A1");
    }

    #[test]
    fn a_formula_giving_nothing_counts_as_filled() {
        let mut sheet = sheet_with(&["A1"]);
        sheet.set(addr("A2"), Some(Cell::formula("\"\"", CellValue::Empty)));
        sheet.set(addr("A3"), Some(Cell::literal(CellValue::Number(3.0))));
        assert_eq!(jump(&sheet, "A1", Direction::Down), "A3");
    }

    #[test]
    fn jumps_stay_fast_on_a_large_sparse_sheet() {
        let mut sheet = Sheet::new("S");
        for row in (0..1_000_000).step_by(1_000) {
            for col in 1..20 {
                sheet.set(CellAddr::new(row, col), Some(Cell::literal(CellValue::Number(1.0))));
            }
        }
        sheet.set(CellAddr::new(999_999, 0), Some(Cell::literal(CellValue::Number(1.0))));
        let started = std::time::Instant::now();
        assert_eq!(jump(&sheet, "A1", Direction::Down), "A1000000");
        assert_eq!(jump(&sheet, "A1000000", Direction::Up), "A1");
        assert_eq!(jump(&sheet, "A1", Direction::Right), "B1");
        let elapsed = started.elapsed();
        assert!(elapsed.as_millis() < 50, "took {elapsed:?}");
    }

    fn hide(sheet: &mut Sheet, rows: &[&str]) {
        let mut meta = sheet.meta().clone();
        for row in rows {
            meta.hidden_rows.insert(row.parse::<u32>().unwrap() - 1);
        }
        sheet.set_meta(meta);
    }

    #[test]
    fn hidden_rows_count_as_empty() {
        let mut sheet = sheet_with(&["A1", "A2", "A3", "A4", "A8"]);
        hide(&mut sheet, &["3"]);
        assert_eq!(jump(&sheet, "A1", Direction::Down), "A4", "a run broken only by a hidden row goes on");
        assert_eq!(jump(&sheet, "A4", Direction::Up), "A1");
        hide(&mut sheet, &["8"]);
        assert_eq!(jump(&sheet, "A4", Direction::Down), "A1048576", "a hidden filled cell is not a stop");
    }

    #[test]
    fn every_row_below_hidden_stops_at_the_last_shown_row() {
        let mut sheet = sheet_with(&["A1"]);
        let mut meta = sheet.meta().clone();
        meta.hidden_rows.insert_run(5, MAX_ROWS - 1);
        sheet.set_meta(meta);
        assert_eq!(jump(&sheet, "A1", Direction::Down), "A5");
        assert_eq!(jump(&sheet, "A5", Direction::Down), "A5");
    }

    #[test]
    fn jumps_stay_fast_with_many_hidden_rows() {
        // A filtered table: 300,000 filled rows with one in a thousand showing.
        let mut sheet = Sheet::new("S");
        for row in 0..300_000 {
            sheet.set(CellAddr::new(row, 0), Some(Cell::literal(CellValue::Number(1.0))));
        }
        let mut meta = sheet.meta().clone();
        for start in (0..300_000).step_by(1000) {
            meta.hidden_rows.insert_run(start + 1, start + 999);
        }
        sheet.set_meta(meta);
        let started = std::time::Instant::now();
        assert_eq!(jump(&sheet, "A1", Direction::Down), "A299001");
        assert_eq!(jump(&sheet, "A299001", Direction::Up), "A1");
        assert_eq!(jump(&sheet, "B1", Direction::Down), "B1048576");
        let elapsed = started.elapsed();
        assert!(elapsed.as_millis() < 50, "took {elapsed:?}");
    }
}
