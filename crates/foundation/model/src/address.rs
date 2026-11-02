use std::fmt;
use std::str::FromStr;

/// Number of rows in an `.xlsx` worksheet.
pub const MAX_ROWS: u32 = 1_048_576;
/// Number of columns in an `.xlsx` worksheet.
pub const MAX_COLS: u32 = 16_384;

/// A zero-based cell position. Ordering is by row, then column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct CellAddr {
    pub row: u32,
    pub col: u32,
}

impl CellAddr {
    pub const fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    /// Returns this address moved by an offset, or `None` if it leaves the sheet.
    pub fn offset(self, rows: i64, cols: i64) -> Option<Self> {
        let row = i64::from(self.row) + rows;
        let col = i64::from(self.col) + cols;
        let valid = (0..i64::from(MAX_ROWS)).contains(&row) && (0..i64::from(MAX_COLS)).contains(&col);
        valid.then(|| Self::new(row as u32, col as u32))
    }
}

impl fmt::Display for CellAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", col_name(self.col), self.row + 1)
    }
}

impl FromStr for CellAddr {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || format!("invalid cell address: {s}");
        let split = s.find(|c: char| !c.is_ascii_alphabetic()).ok_or_else(invalid)?;
        let (letters, digits) = s.split_at(split);
        let col = col_from_name(letters).ok_or_else(invalid)?;
        if !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid());
        }
        let row: u32 = digits.parse().map_err(|_| invalid())?;
        if row == 0 || row > MAX_ROWS {
            return Err(invalid());
        }
        Ok(Self::new(row - 1, col))
    }
}

/// Converts a zero-based column index to its letters, such as `0` to `A`.
pub fn col_name(col: u32) -> String {
    let mut n = col + 1;
    let mut letters = Vec::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        letters.push(b'A' + rem as u8);
        n = (n - 1) / 26;
    }
    letters.reverse();
    String::from_utf8(letters).expect("column letters are ASCII")
}

/// Converts column letters to a zero-based index, such as `A` to `0`.
pub fn col_from_name(name: &str) -> Option<u32> {
    if name.is_empty() || name.len() > 3 {
        return None;
    }
    let mut col: u32 = 0;
    for b in name.bytes() {
        if !b.is_ascii_alphabetic() {
            return None;
        }
        col = col * 26 + u32::from(b.to_ascii_uppercase() - b'A') + 1;
    }
    (col <= MAX_COLS).then(|| col - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_names_round_trip() {
        for (col, name) in [(0, "A"), (25, "Z"), (26, "AA"), (701, "ZZ"), (702, "AAA"), (16383, "XFD")] {
            assert_eq!(col_name(col), name);
            assert_eq!(col_from_name(name), Some(col));
        }
    }

    #[test]
    fn rejects_columns_past_the_limit() {
        assert_eq!(col_from_name("XFE"), None);
        assert_eq!(col_from_name(""), None);
        assert_eq!(col_from_name("A1"), None);
    }

    #[test]
    fn parses_and_formats_addresses() {
        let addr: CellAddr = "b5".parse().unwrap();
        assert_eq!(addr, CellAddr::new(4, 1));
        assert_eq!(addr.to_string(), "B5");
    }

    #[test]
    fn rejects_invalid_addresses() {
        for bad in ["", "A", "5", "A0", "A1048577", "A1B", "A-1", "가1"] {
            assert!(bad.parse::<CellAddr>().is_err(), "{bad}");
        }
    }

    #[test]
    fn offset_stops_at_sheet_edges() {
        assert_eq!(CellAddr::new(1, 1).offset(1, -1), Some(CellAddr::new(2, 0)));
        assert_eq!(CellAddr::new(0, 0).offset(-1, 0), None);
        assert_eq!(CellAddr::new(MAX_ROWS - 1, 0).offset(1, 0), None);
    }
}
