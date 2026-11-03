pub(crate) mod hlookup;
pub(crate) mod index;
pub(crate) mod match_fn;
pub(crate) mod vlookup;
pub(crate) mod xlookup;

use std::cmp::Ordering;

use excel_model::{CellError, CellValue};

use crate::Cells;

/// Compares values of the same kind: numbers, text ignoring case, or
/// booleans. Different kinds, blanks, and errors do not compare.
pub(crate) fn compare(a: &CellValue, b: &CellValue) -> Option<Ordering> {
    match (a, b) {
        (CellValue::Number(x), CellValue::Number(y)) => x.partial_cmp(y),
        (CellValue::Text(x), CellValue::Text(y)) => Some(x.to_lowercase().cmp(&y.to_lowercase())),
        (CellValue::Bool(x), CellValue::Bool(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

/// Whether `value` equals `wanted`. With `wildcards`, text may use `*` and `?`
/// as in conditions.
pub(crate) fn equals(wanted: &CellValue, value: &CellValue, wildcards: bool) -> bool {
    match (wanted, value) {
        (CellValue::Text(pattern), CellValue::Text(_)) if wildcards && pattern.contains(['*', '?', '~']) => {
            excel_model::Criterion::new(wanted).matches(value)
        }
        _ => compare(wanted, value) == Some(Ordering::Equal),
    }
}

/// The values along one line of cells, as positions and values, with empty
/// cells left out. `across` reads along a row instead of a column.
pub(crate) fn line(cells: &Cells, fixed: u32, across: bool) -> Vec<(u32, &CellValue)> {
    cells
        .entries()
        .iter()
        .filter(|(row, col, _)| if across { *row == fixed } else { *col == fixed })
        .map(|(row, col, value)| (if across { *col } else { *row }, value))
        .collect()
}

/// How a search treats a value that is not found exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Only an exact match, with wildcards in text.
    Exact,
    /// The largest value not above the one wanted, for data sorted ascending.
    NotAbove,
    /// The smallest value not below the one wanted, for data sorted descending.
    NotBelow,
}

/// Finds `wanted` along a line, giving its position. Searches for the nearest
/// value scan in order and stop once the data passes the value wanted, which
/// gives what Excel's search gives on sorted data.
pub(crate) fn search(wanted: &CellValue, line: &[(u32, &CellValue)], mode: Mode) -> Result<u32, CellError> {
    if let CellValue::Error(error) = wanted {
        return Err(*error);
    }
    match mode {
        Mode::Exact => line.iter().find(|(_, value)| equals(wanted, value, true)).map(|(at, _)| *at),
        Mode::NotAbove | Mode::NotBelow => {
            let passed = if mode == Mode::NotAbove { Ordering::Greater } else { Ordering::Less };
            let mut best = None;
            for (at, value) in line {
                match compare(value, wanted) {
                    Some(Ordering::Equal) => return Ok(*at),
                    Some(order) if order == passed => break,
                    Some(_) => best = Some(*at),
                    None => {}
                }
            }
            best
        }
    }
    .ok_or(CellError::Na)
}

/// A 1-based position from a number, as a 0-based index below `limit`.
/// Below 1 is `#VALUE!`, and past the end is `#REF!`.
pub(crate) fn position(n: f64, limit: u32) -> Result<u32, CellError> {
    let n = n.trunc();
    if n < 1.0 {
        Err(CellError::Value)
    } else if n > f64::from(limit) {
        Err(CellError::Ref)
    } else {
        Ok(n as u32 - 1)
    }
}

/// A value read from a cell, where an empty cell gives 0 as a formula shows it.
pub(crate) fn shown(value: &CellValue) -> CellValue {
    match value {
        CellValue::Empty => CellValue::Number(0.0),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(value: f64) -> CellValue {
        CellValue::Number(value)
    }

    #[test]
    fn exact_and_nearest_searches() {
        let values = [n(10.0), n(20.0), n(30.0)];
        let line: Vec<(u32, &CellValue)> = values.iter().enumerate().map(|(i, v)| (i as u32, v)).collect();
        assert_eq!(search(&n(20.0), &line, Mode::Exact), Ok(1));
        assert_eq!(search(&n(25.0), &line, Mode::Exact), Err(CellError::Na));
        assert_eq!(search(&n(25.0), &line, Mode::NotAbove), Ok(1));
        assert_eq!(search(&n(99.0), &line, Mode::NotAbove), Ok(2));
        assert_eq!(search(&n(5.0), &line, Mode::NotAbove), Err(CellError::Na));

        let falling = [n(30.0), n(20.0), n(10.0)];
        let line: Vec<(u32, &CellValue)> = falling.iter().enumerate().map(|(i, v)| (i as u32, v)).collect();
        assert_eq!(search(&n(25.0), &line, Mode::NotBelow), Ok(0));
        assert_eq!(search(&n(15.0), &line, Mode::NotBelow), Ok(1));
    }

    #[test]
    fn text_ignores_case_and_allows_wildcards() {
        let values = [CellValue::Text("Apple".into()), CellValue::Text("사과".into())];
        let line: Vec<(u32, &CellValue)> = values.iter().enumerate().map(|(i, v)| (i as u32, v)).collect();
        assert_eq!(search(&CellValue::Text("apple".into()), &line, Mode::Exact), Ok(0));
        assert_eq!(search(&CellValue::Text("사*".into()), &line, Mode::Exact), Ok(1));
        assert_eq!(search(&n(1.0), &line, Mode::Exact), Err(CellError::Na), "a number is not text");
    }

    #[test]
    fn positions() {
        assert_eq!(position(2.9, 3), Ok(1));
        assert_eq!(position(0.0, 3), Err(CellError::Value));
        assert_eq!(position(4.0, 3), Err(CellError::Ref));
    }
}
