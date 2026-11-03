//! A set of cells kept as ranges that never overlap, for input cells.

use crate::{CellAddr, CellRange};

/// Cells kept as ranges that never overlap. Adding and removing split and
/// join ranges as needed, so empty cells count too and a large block stays one
/// range.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RangeSet {
    ranges: Vec<CellRange>,
}

impl RangeSet {
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// The ranges, top to bottom, then left to right.
    pub fn ranges(&self) -> &[CellRange] {
        &self.ranges
    }

    pub fn contains(&self, addr: CellAddr) -> bool {
        self.ranges.iter().any(|range| range.contains(addr))
    }

    /// The ranges that share cells with `area`.
    pub fn overlapping(&self, area: CellRange) -> impl Iterator<Item = CellRange> + '_ {
        self.ranges.iter().copied().filter(move |range| range.overlaps(area))
    }

    /// The next cell in the set after `from`, reading left to right and then
    /// down, or the one before it going back. Wraps around at the ends, as
    /// Excel does on a protected sheet. `None` when the set is empty.
    pub fn next_after(&self, from: CellAddr, forward: bool) -> Option<CellAddr> {
        let key = |addr: CellAddr| (addr.row, addr.col);
        let candidates = self
            .ranges
            .iter()
            .filter_map(|range| if forward { first_after(*range, from) } else { last_before(*range, from) });
        let found = if forward { candidates.min_by_key(|a| key(*a)) } else { candidates.max_by_key(|a| key(*a)) };
        found.or_else(|| {
            let ends = self.ranges.iter().map(|range| if forward { range.start } else { range.end });
            if forward { ends.min_by_key(|a| key(*a)) } else { ends.max_by_key(|a| key(*a)) }
        })
    }

    pub fn add(&mut self, range: CellRange) {
        self.remove(range);
        self.ranges.push(range);
        self.tidy();
    }

    pub fn remove(&mut self, cut: CellRange) {
        self.ranges = self.ranges.iter().flat_map(|range| subtract(*range, cut)).collect();
        self.tidy();
    }

    pub fn clear(&mut self) {
        self.ranges.clear();
    }

    /// Moves every range with `map`, which gives where a range is now, or
    /// `None` when it is gone.
    pub(crate) fn remap(&mut self, map: impl Fn(CellRange) -> Option<CellRange>) {
        self.ranges = self.ranges.iter().filter_map(|range| map(*range)).collect();
        self.tidy();
    }

    /// Joins ranges that sit side by side with the same height, or one above
    /// the other with the same width, and sorts them.
    fn tidy(&mut self) {
        loop {
            let mut joined = false;
            'outer: for i in 0..self.ranges.len() {
                for j in 0..self.ranges.len() {
                    if i != j
                        && let Some(both) = join(self.ranges[i], self.ranges[j])
                    {
                        self.ranges[i] = both;
                        self.ranges.remove(j);
                        joined = true;
                        break 'outer;
                    }
                }
            }
            if !joined {
                break;
            }
        }
        self.ranges.sort_by_key(|range| (range.start, range.end));
    }
}

impl FromIterator<CellRange> for RangeSet {
    fn from_iter<I: IntoIterator<Item = CellRange>>(ranges: I) -> Self {
        let mut set = Self::default();
        for range in ranges {
            set.add(range);
        }
        set
    }
}

/// The first cell of `range` after `from`, row by row.
fn first_after(range: CellRange, from: CellAddr) -> Option<CellAddr> {
    if (range.start.row..=range.end.row).contains(&from.row) && from.col < range.end.col {
        return Some(CellAddr::new(from.row, (from.col + 1).max(range.start.col)));
    }
    let row = range.start.row.max(from.row + 1);
    (row <= range.end.row).then(|| CellAddr::new(row, range.start.col))
}

/// The last cell of `range` before `from`, row by row.
fn last_before(range: CellRange, from: CellAddr) -> Option<CellAddr> {
    if (range.start.row..=range.end.row).contains(&from.row) && from.col > range.start.col {
        return Some(CellAddr::new(from.row, (from.col - 1).min(range.end.col)));
    }
    let row = range.end.row.min(from.row.checked_sub(1)?);
    (row >= range.start.row).then(|| CellAddr::new(row, range.end.col))
}

/// `a` joined with `b` when together they make one rectangle.
fn join(a: CellRange, b: CellRange) -> Option<CellRange> {
    let same_cols = a.start.col == b.start.col && a.end.col == b.end.col;
    let same_rows = a.start.row == b.start.row && a.end.row == b.end.row;
    if same_cols && a.end.row.checked_add(1) == Some(b.start.row) {
        return Some(CellRange { start: a.start, end: b.end });
    }
    if same_rows && a.end.col.checked_add(1) == Some(b.start.col) {
        return Some(CellRange { start: a.start, end: b.end });
    }
    None
}

/// What is left of `range` once `cut` is taken out: up to four ranges, the
/// rows above and below the cut, then the parts left and right of it.
fn subtract(range: CellRange, cut: CellRange) -> Vec<CellRange> {
    if !range.overlaps(cut) {
        return vec![range];
    }
    let mut left = Vec::new();
    let (top, bottom) = (range.start.row.max(cut.start.row), range.end.row.min(cut.end.row));
    if range.start.row < cut.start.row {
        left.push(CellRange { start: range.start, end: CellAddr::new(cut.start.row - 1, range.end.col) });
    }
    if range.end.row > cut.end.row {
        left.push(CellRange { start: CellAddr::new(cut.end.row + 1, range.start.col), end: range.end });
    }
    if range.start.col < cut.start.col {
        left.push(CellRange {
            start: CellAddr::new(top, range.start.col),
            end: CellAddr::new(bottom, cut.start.col - 1),
        });
    }
    if range.end.col > cut.end.col {
        left.push(CellRange { start: CellAddr::new(top, cut.end.col + 1), end: CellAddr::new(bottom, range.end.col) });
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ranges: &[&str]) -> RangeSet {
        ranges.iter().map(|text| text.parse().unwrap()).collect()
    }

    fn names(set: &RangeSet) -> Vec<String> {
        set.ranges().iter().map(|range| range.to_string()).collect()
    }

    #[test]
    fn adding_joins_and_never_overlaps() {
        let mut inputs = set(&["B2:B4", "C2:C4"]);
        assert_eq!(names(&inputs), ["B2:C4"]);
        inputs.add("B5:C5".parse().unwrap());
        assert_eq!(names(&inputs), ["B2:C5"]);
        inputs.add("C3:D3".parse().unwrap());
        assert!(inputs.contains("D3".parse().unwrap()) && inputs.contains("B2".parse().unwrap()));
        let cells: usize = inputs.ranges().iter().map(|r| (r.rows() * r.cols()) as usize).sum();
        assert_eq!(cells, 9, "eight cells of B2:C5 and D3, none counted twice");
    }

    #[test]
    fn steps_through_cells_in_reading_order_and_wraps() {
        let inputs = set(&["B2:C3", "E2", "A5"]);
        let walk = |from: &str, forward: bool| inputs.next_after(from.parse().unwrap(), forward).unwrap().to_string();
        assert_eq!(walk("A1", true), "B2");
        assert_eq!(walk("B2", true), "C2");
        assert_eq!(walk("C2", true), "E2");
        assert_eq!(walk("E2", true), "B3");
        assert_eq!(walk("C3", true), "A5");
        assert_eq!(walk("A5", true), "B2", "wraps to the first");
        assert_eq!(walk("B3", false), "E2");
        assert_eq!(walk("B2", false), "A5", "wraps to the last");
        assert_eq!(walk("D9", false), "A5");
        assert!(RangeSet::default().next_after("A1".parse().unwrap(), true).is_none());
    }

    #[test]
    fn removing_splits() {
        let mut inputs = set(&["A1:C3"]);
        inputs.remove("B2".parse().unwrap());
        assert_eq!(names(&inputs), ["A1:C1", "A2", "C2", "A3:C3"]);
        assert!(!inputs.contains("B2".parse().unwrap()));
        inputs.add("B2".parse().unwrap());
        assert_eq!(names(&inputs), ["A1:C3"], "filling the hole joins it back");
        inputs.remove("A1:Z9".parse().unwrap());
        assert!(inputs.is_empty());
    }
}
