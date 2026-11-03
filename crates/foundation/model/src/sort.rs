//! Sorting rows, shared by the `sort` and `table sort` commands.

use std::cmp::Ordering;

use crate::{Cell, CellAddr, CellInput, CellRange, CellValue, Sheet};

/// Orders values as Excel does: numbers, text, booleans, errors, with text
/// ignoring case. Empty cells come last in either direction.
pub fn sort_order(a: &CellValue, b: &CellValue, descending: bool) -> Ordering {
    match (a.is_empty(), b.is_empty()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        (false, false) => {}
    }
    let ordering = match (a, b) {
        (CellValue::Number(x), CellValue::Number(y)) => x.total_cmp(y),
        (CellValue::Text(x), CellValue::Text(y)) => x.to_lowercase().cmp(&y.to_lowercase()),
        (CellValue::Bool(x), CellValue::Bool(y)) => x.cmp(y),
        _ => rank(a).cmp(&rank(b)),
    };
    if descending { ordering.reverse() } else { ordering }
}

fn rank(value: &CellValue) -> u8 {
    match value {
        CellValue::Number(_) => 0,
        CellValue::Text(_) => 1,
        CellValue::Bool(_) => 2,
        CellValue::Error(_) => 3,
        CellValue::Empty => 4,
    }
}

/// A row being sorted: where it came from, its sort key, and its cells by column.
type Row = (u32, CellValue, Vec<(u32, Cell)>);

impl Sheet {
    /// Sorts the rows of `range` by the values in column `col`. The sort is
    /// stable, so rows with equal keys keep their order. Formulas move with
    /// their rows: `translate` rewrites one for a move by some number of rows.
    pub fn sort_rows(&mut self, range: CellRange, col: u32, descending: bool, translate: impl Fn(&str, i64) -> String) {
        let mut rows: Vec<Row> = (range.start.row..=range.end.row)
            .map(|row| {
                let row_range =
                    CellRange { start: CellAddr::new(row, range.start.col), end: CellAddr::new(row, range.end.col) };
                let cells = self.iter_range(row_range).map(|(addr, cell)| (addr.col, cell.clone())).collect();
                (row, self.value(CellAddr::new(row, col)).clone(), cells)
            })
            .collect();
        rows.sort_by(|a, b| sort_order(&a.1, &b.1, descending));

        for addr in range.iter() {
            self.set(addr, None);
        }
        for (index, (source_row, _, cells)) in rows.into_iter().enumerate() {
            let row = range.start.row + index as u32;
            let offset = i64::from(row) - i64::from(source_row);
            for (cell_col, mut cell) in cells {
                if let CellInput::Formula(text) = &cell.input {
                    cell.input = CellInput::Formula(translate(text, offset));
                }
                self.set(CellAddr::new(row, cell_col), Some(cell));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CellError;

    #[test]
    fn errors_sort_after_booleans_and_empties_last() {
        assert_eq!(sort_order(&CellValue::Bool(true), &CellValue::Error(CellError::Na), false), Ordering::Less);
        assert_eq!(sort_order(&CellValue::Empty, &CellValue::Number(1.0), true), Ordering::Greater);
        assert_eq!(sort_order(&CellValue::Text("b".into()), &CellValue::Text("A".into()), false), Ordering::Greater);
    }
}
