//! Finding positions that meet several conditions, for `COUNTIFS` and the
//! functions like it. The conditions themselves are `excel_model::Criterion`.

use excel_model::{CellValue, Criterion};

use crate::Cells;

/// Positions in `shape` meeting every `(cells, criterion)` pair, which must all
/// have that shape. Only positions with a value in some range can fail a
/// condition that blank cells meet, so the search starts from the values of
/// a range whose condition blanks fail, and covers every cell otherwise.
pub(crate) fn matching_positions(shape: (u32, u32), pairs: &[(&Cells, Criterion)]) -> Vec<(u32, u32)> {
    let meets = |row: u32, col: u32| pairs.iter().all(|(cells, criterion)| criterion.matches(cells.get(row, col)));
    match pairs.iter().find(|(_, criterion)| !criterion.matches(&CellValue::Empty)) {
        Some((cells, _)) => {
            cells.entries().iter().map(|(row, col, _)| (*row, *col)).filter(|(row, col)| meets(*row, *col)).collect()
        }
        None => (0..shape.0)
            .flat_map(|row| (0..shape.1).map(move |col| (row, col)))
            .filter(|(r, c)| meets(*r, *c))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use excel_model::CellValue;

    use super::*;

    fn n(value: f64) -> CellValue {
        CellValue::Number(value)
    }

    fn t(text: &str) -> CellValue {
        CellValue::Text(text.into())
    }

    #[test]
    fn finds_positions_meeting_every_condition() {
        let names = Cells::new(3, 1, [(0, 0, t("a")), (1, 0, t("b")), (2, 0, t("a"))]);
        let sizes = Cells::new(3, 1, [(0, 0, n(1.0)), (2, 0, n(9.0))]);
        let pairs = [(&names, Criterion::new(&t("a"))), (&sizes, Criterion::new(&t(">5")))];
        assert_eq!(matching_positions((3, 1), &pairs), [(2, 0)]);
        let blanks = [(&sizes, Criterion::new(&t("")))];
        assert_eq!(matching_positions((3, 1), &blanks), [(1, 0)]);
    }
}
