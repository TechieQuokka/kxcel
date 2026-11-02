use std::fmt;
use std::str::FromStr;

use crate::CellAddr;

/// A rectangular range. `start` is always the top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellRange {
    pub start: CellAddr,
    pub end: CellAddr,
}

impl CellRange {
    /// Builds a normalized range from any two opposite corners.
    pub fn new(a: CellAddr, b: CellAddr) -> Self {
        Self {
            start: CellAddr::new(a.row.min(b.row), a.col.min(b.col)),
            end: CellAddr::new(a.row.max(b.row), a.col.max(b.col)),
        }
    }

    pub fn single(addr: CellAddr) -> Self {
        Self { start: addr, end: addr }
    }

    pub fn is_single(&self) -> bool {
        self.start == self.end
    }

    pub fn contains(&self, addr: CellAddr) -> bool {
        (self.start.row..=self.end.row).contains(&addr.row) && (self.start.col..=self.end.col).contains(&addr.col)
    }

    pub fn rows(&self) -> u32 {
        self.end.row - self.start.row + 1
    }

    pub fn cols(&self) -> u32 {
        self.end.col - self.start.col + 1
    }

    /// Visits each address, row by row.
    pub fn iter(&self) -> impl Iterator<Item = CellAddr> + use<> {
        let Self { start, end } = *self;
        (start.row..=end.row).flat_map(move |row| (start.col..=end.col).map(move |col| CellAddr::new(row, col)))
    }
}

impl fmt::Display for CellRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_single() { write!(f, "{}", self.start) } else { write!(f, "{}:{}", self.start, self.end) }
    }
}

impl FromStr for CellRange {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once(':') {
            Some((a, b)) => Ok(Self::new(a.parse()?, b.parse()?)),
            None => Ok(Self::single(s.parse()?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_corners() {
        let range: CellRange = "C3:A1".parse().unwrap();
        assert_eq!(range.to_string(), "A1:C3");
        assert_eq!((range.rows(), range.cols()), (3, 3));
    }

    #[test]
    fn iterates_row_by_row() {
        let range: CellRange = "A1:B2".parse().unwrap();
        let cells: Vec<_> = range.iter().map(|a| a.to_string()).collect();
        assert_eq!(cells, ["A1", "B1", "A2", "B2"]);
    }

    #[test]
    fn contains_only_inner_cells() {
        let range: CellRange = "B2:C3".parse().unwrap();
        assert!(range.contains("C3".parse().unwrap()));
        assert!(!range.contains("D3".parse().unwrap()));
    }
}
