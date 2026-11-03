//! A set of rows kept as runs, so a long run of hidden rows is cheap to skip.

use std::collections::BTreeMap;

use crate::MAX_ROWS;

/// Rows, stored as runs of consecutive rows: first row to last row. Runs never
/// touch or overlap, so equal sets compare equal.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowSet {
    runs: BTreeMap<u32, u32>,
}

impl RowSet {
    pub const fn new() -> Self {
        Self { runs: BTreeMap::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }

    /// How many rows are in the set.
    pub fn len(&self) -> u64 {
        self.runs.iter().map(|(first, last)| u64::from(last - first) + 1).sum()
    }

    pub fn contains(&self, row: u32) -> bool {
        self.run_at(row).is_some()
    }

    /// The run holding `row`.
    fn run_at(&self, row: u32) -> Option<(u32, u32)> {
        self.runs.range(..=row).next_back().filter(|(_, last)| **last >= row).map(|(first, last)| (*first, *last))
    }

    /// Every row, in order.
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.runs.iter().flat_map(|(first, last)| *first..=*last)
    }

    /// How many of the rows from `first` to `last` are in the set.
    pub fn count_within(&self, first: u32, last: u32) -> u64 {
        let before = self.runs.range(..first).next_back().map(|(_, end)| (first, *end));
        before
            .into_iter()
            .chain(self.runs.range(first..=last).map(|(start, end)| (*start, *end)))
            .map(|(start, end)| (start.max(first), end.min(last)))
            .filter(|(start, end)| start <= end)
            .map(|(start, end)| u64::from(end - start) + 1)
            .sum()
    }

    /// `row` if it is not in the set, else the nearest row past its run going
    /// down, or up when `down` is false. `None` when the run reaches the sheet's edge.
    pub fn skip(&self, row: u32, down: bool) -> Option<u32> {
        match self.run_at(row) {
            None => Some(row),
            Some((_, last)) if down => (last + 1 < MAX_ROWS).then_some(last + 1),
            Some((first, _)) => first.checked_sub(1),
        }
    }

    pub fn insert(&mut self, row: u32) {
        self.insert_run(row, row);
    }

    /// Adds every row from `first` to `last`, joining runs that touch.
    pub fn insert_run(&mut self, mut first: u32, mut last: u32) {
        if let Some((start, end)) = self.runs.range(..=first).next_back().map(|(s, e)| (*s, *e))
            && end.saturating_add(1) >= first
        {
            first = start;
            last = last.max(end);
        }
        let swallowed: Vec<(u32, u32)> =
            self.runs.range(first..=last.saturating_add(1)).map(|(start, end)| (*start, *end)).collect();
        for (start, end) in swallowed {
            self.runs.remove(&start);
            last = last.max(end);
        }
        self.runs.insert(first, last);
    }

    /// Moves rows after `count` were inserted at `at`. Inserted rows are not in
    /// the set, and rows pushed off the sheet are dropped.
    pub fn insert_rows(&mut self, at: u32, count: u32) {
        let shift = |row: u32| row.checked_add(count).filter(|row| *row < MAX_ROWS);
        let mut moved = RowSet::default();
        for (first, last) in std::mem::take(&mut self.runs) {
            if last < at {
                moved.insert_run(first, last);
                continue;
            }
            if first < at {
                moved.insert_run(first, at - 1);
            }
            if let Some(start) = shift(first.max(at)) {
                moved.insert_run(start, shift(last).unwrap_or(MAX_ROWS - 1));
            }
        }
        *self = moved;
    }

    /// Removes `count` rows at `at` and moves later rows up.
    pub fn delete_rows(&mut self, at: u32, count: u32) {
        let end = at.saturating_add(count);
        let mut moved = RowSet::default();
        for (first, last) in std::mem::take(&mut self.runs) {
            if first < at {
                moved.insert_run(first, last.min(at - 1));
            }
            if last >= end {
                let start = first.max(end);
                moved.insert_run(start - count, last - count);
            }
        }
        *self = moved;
    }
}

impl FromIterator<u32> for RowSet {
    fn from_iter<I: IntoIterator<Item = u32>>(rows: I) -> Self {
        let mut set = Self::default();
        for row in rows {
            set.insert(row);
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(set: &RowSet) -> Vec<u32> {
        set.iter().collect()
    }

    #[test]
    fn runs_join_and_compare_by_content() {
        let set: RowSet = [5, 3, 4, 9, 10, 7].into_iter().collect();
        assert_eq!(rows(&set), [3, 4, 5, 7, 9, 10]);
        assert_eq!(set.runs.len(), 3);
        let mut other: RowSet = [7, 9, 10].into_iter().collect();
        other.insert_run(3, 5);
        assert_eq!(set, other);
        other.insert(6);
        other.insert(8);
        assert_eq!(other.runs.len(), 1);
        assert_eq!(other.len(), 8);
    }

    #[test]
    fn contains_skip_and_count() {
        let set: RowSet = [2, 3, 4, 8].into_iter().collect();
        assert!(set.contains(3) && !set.contains(5));
        assert_eq!(set.skip(3, true), Some(5));
        assert_eq!(set.skip(3, false), Some(1));
        assert_eq!(set.skip(5, true), Some(5));
        assert_eq!(set.count_within(3, 8), 3);
        assert_eq!(set.count_within(0, 1), 0);
        let edge: RowSet = [0, 1, MAX_ROWS - 1].into_iter().collect();
        assert_eq!(edge.skip(1, false), None);
        assert_eq!(edge.skip(MAX_ROWS - 1, true), None);
    }

    #[test]
    fn inserting_rows_splits_runs() {
        let mut set: RowSet = [2, 3, 4, 8].into_iter().collect();
        set.insert_rows(3, 2);
        assert_eq!(rows(&set), [2, 5, 6, 10]);
    }

    #[test]
    fn deleting_rows_trims_and_joins_runs() {
        let mut set: RowSet = [2, 3, 4, 6, 7].into_iter().collect();
        set.delete_rows(4, 2);
        assert_eq!(rows(&set), [2, 3, 4, 5]);
        assert_eq!(set.runs.len(), 1, "the runs either side of the gap meet");
        set.delete_rows(0, 3);
        assert_eq!(rows(&set), [0, 1, 2]);
    }

    #[test]
    fn a_huge_run_is_cheap() {
        let mut set = RowSet::default();
        set.insert_run(10, 1_000_000);
        assert_eq!(set.skip(10, true), Some(1_000_001));
        assert_eq!(set.len(), 999_991);
    }
}
