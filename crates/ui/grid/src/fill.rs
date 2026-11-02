use excel_model::{CellAddr, CellRange};

/// The range a fill handle drag covers.
///
/// Dragging outward extends `source` in one direction only, whichever one the
/// pointer has moved furthest beyond it. Dragging back inside shrinks it from
/// the bottom or right, and the cells left out are cleared, as in Excel.
pub fn fill_target(source: CellRange, pointer: CellAddr) -> CellRange {
    let below = i64::from(pointer.row) - i64::from(source.end.row);
    let above = i64::from(source.start.row) - i64::from(pointer.row);
    let right = i64::from(pointer.col) - i64::from(source.end.col);
    let left = i64::from(source.start.col) - i64::from(pointer.col);

    let vertical = below.max(above);
    let horizontal = right.max(left);
    if vertical <= 0 && horizontal <= 0 {
        return shrink_target(source, pointer);
    }

    let (mut start, mut end) = (source.start, source.end);
    if vertical >= horizontal {
        if below > 0 {
            end.row = pointer.row;
        } else {
            start.row = pointer.row;
        }
    } else if right > 0 {
        end.col = pointer.col;
    } else {
        start.col = pointer.col;
    }
    CellRange { start, end }
}

/// The range kept when the pointer is back inside `source`.
fn shrink_target(source: CellRange, pointer: CellAddr) -> CellRange {
    let rows_cut = source.end.row - pointer.row;
    let cols_cut = source.end.col - pointer.col;
    let mut kept = source;
    if rows_cut >= cols_cut && rows_cut > 0 {
        kept.end.row = pointer.row;
    } else if cols_cut > 0 {
        kept.end.col = pointer.col;
    }
    kept
}

/// The cells a shrinking fill clears, or `None` if `target` does not shrink `source`.
pub fn shrunk_away(source: CellRange, target: CellRange) -> Option<CellRange> {
    if target == source || !(source.contains(target.start) && source.contains(target.end)) {
        return None;
    }
    let start = if target.end.row < source.end.row {
        CellAddr::new(target.end.row + 1, source.start.col)
    } else {
        CellAddr::new(source.start.row, target.end.col + 1)
    };
    Some(CellRange { start, end: source.end })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(source: &str, pointer: &str) -> String {
        fill_target(source.parse().unwrap(), pointer.parse().unwrap()).to_string()
    }

    #[test]
    fn extends_in_each_direction() {
        assert_eq!(target("B2:C3", "B6"), "B2:C6");
        assert_eq!(target("B3:C4", "B1"), "B1:C4");
        assert_eq!(target("B2:C3", "F2"), "B2:F3");
        assert_eq!(target("C2:D3", "A2"), "A2:D3");
    }

    #[test]
    fn pointer_on_the_corner_changes_nothing() {
        assert_eq!(target("B2:C3", "C3"), "B2:C3");
    }

    #[test]
    fn uses_the_dominant_direction() {
        assert_eq!(target("A1", "B5"), "A1:A5");
        assert_eq!(target("A1", "E2"), "A1:E1");
    }

    #[test]
    fn shrinks_from_the_bottom_or_right() {
        assert_eq!(target("A1:A5", "A3"), "A1:A3");
        assert_eq!(target("A1:E1", "B1"), "A1:B1");
        assert_eq!(target("A1:C5", "C2"), "A1:C2");
        assert_eq!(target("A1:C5", "A5"), "A1:A5");
    }

    #[test]
    fn shrinking_keeps_at_least_the_first_row_or_column() {
        assert_eq!(target("A1:A5", "A1"), "A1");
    }

    #[test]
    fn reports_the_cells_a_shrink_clears() {
        let cut = |source: &str, target: &str| {
            shrunk_away(source.parse().unwrap(), target.parse().unwrap()).map(|range| range.to_string())
        };
        assert_eq!(cut("A1:A5", "A1:A3").as_deref(), Some("A4:A5"));
        assert_eq!(cut("A1:E2", "A1:B2").as_deref(), Some("C1:E2"));
        assert_eq!(cut("A1:A5", "A1:A5"), None);
        assert_eq!(cut("A1:A5", "A1:A9"), None, "growing clears nothing");
    }
}
