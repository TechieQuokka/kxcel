//! Detects a series in the cells a fill starts from, so a fill can continue it.

use excel_model::{Cell, CellInput, CellValue};

/// How a fill continues a line of cells.
#[derive(Debug, Clone, PartialEq)]
pub enum Series {
    /// Numbers with a constant difference: 1, 2, 3.
    Arithmetic { first: f64, step: f64 },
    /// Numbers with a constant ratio: 1, 2, 4.
    Geometric { first: f64, ratio: f64 },
    /// Text ending in a number, which counts: Item1, Item2.
    Numbered { prefix: String, first: i64, step: i64, digits: usize },
}

/// Finds the series in a line of cells, in order. `None` means the cells are
/// copied as they are.
///
/// A single number copies, and a single numbered text counts up, as in Excel.
/// `alternate`, which Ctrl gives, swaps copying and continuing.
pub fn detect(cells: &[Option<&Cell>], alternate: bool) -> Option<Series> {
    let series = detect_numbers(cells, alternate).or_else(|| detect_numbered_text(cells, alternate));
    if alternate && cells.len() > 1 { None } else { series }
}

impl Series {
    /// The cell at position `n`, counted from the first source cell. Positions
    /// before the source are negative. `None` means the series cannot go there,
    /// such as a value too large to represent, so the caller copies instead.
    pub fn cell_at(&self, n: i64) -> Option<Cell> {
        match self {
            Self::Arithmetic { first, step } => number(first + step * n as f64),
            Self::Geometric { first, ratio } => number(first * ratio.powf(n as f64)),
            Self::Numbered { prefix, first, step, digits } => {
                let value = first.checked_add(step.checked_mul(n)?)?;
                (value >= 0).then(|| Cell::literal(CellValue::Text(format!("{prefix}{value:0digits$}"))))
            }
        }
    }
}

/// A number cell, rounded to 15 significant digits so that steps such as 0.1
/// do not pile up binary rounding noise.
fn number(value: f64) -> Option<Cell> {
    let rounded: f64 = format!("{value:.14e}").parse().ok()?;
    rounded.is_finite().then(|| Cell::literal(CellValue::Number(rounded)))
}

fn literal_number(cell: Option<&Cell>) -> Option<f64> {
    match &cell?.input {
        CellInput::Literal(CellValue::Number(n)) => Some(*n),
        _ => None,
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn detect_numbers(cells: &[Option<&Cell>], alternate: bool) -> Option<Series> {
    let values: Vec<f64> = cells.iter().map(|cell| literal_number(*cell)).collect::<Option<_>>()?;
    if let [first] = values[..] {
        return alternate.then_some(Series::Arithmetic { first, step: 1.0 });
    }
    let step = values[1] - values[0];
    if values.windows(2).all(|pair| close(pair[1] - pair[0], step)) {
        return Some(Series::Arithmetic { first: values[0], step });
    }
    // Two numbers always form an arithmetic series, so a ratio needs three.
    if values.len() >= 3 && values.iter().all(|value| *value != 0.0) {
        let ratio = values[1] / values[0];
        if values.windows(2).all(|pair| close(pair[1] / pair[0], ratio)) {
            return Some(Series::Geometric { first: values[0], ratio });
        }
    }
    None
}

/// Splits text such as `Item07` into `Item`, 7, and 2 digits.
fn split_number(text: &str) -> Option<(&str, i64, usize)> {
    let digits_start = text.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    let (prefix, digits) = text.split_at(digits_start);
    if prefix.is_empty() || digits.is_empty() {
        return None;
    }
    Some((prefix, digits.parse().ok()?, digits.len()))
}

fn detect_numbered_text(cells: &[Option<&Cell>], alternate: bool) -> Option<Series> {
    let parts: Vec<(&str, i64, usize)> = cells
        .iter()
        .map(|cell| match &(*cell)?.input {
            CellInput::Literal(CellValue::Text(text)) => split_number(text),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let (prefix, first, digits) = parts[0];
    if parts.iter().any(|(other, _, _)| *other != prefix) {
        return None;
    }
    let step = match parts.get(1) {
        Some((_, second, _)) => second - first,
        None if alternate => return None,
        None => 1,
    };
    let counts = parts.windows(2).all(|pair| pair[1].1 - pair[0].1 == step);
    counts.then(|| Series::Numbered { prefix: prefix.to_owned(), first, step, digits })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Continues typed values `count` places after them.
    fn continued(typed: &[&str], count: i64, alternate: bool) -> Option<Vec<String>> {
        let cells: Vec<Cell> = typed.iter().map(|text| Cell::parse_input(text).unwrap()).collect();
        let refs: Vec<Option<&Cell>> = cells.iter().map(Some).collect();
        let series = detect(&refs, alternate)?;
        let start = typed.len() as i64;
        Some((start..start + count).map(|n| series.cell_at(n).unwrap().value.to_string()).collect())
    }

    #[test]
    fn continues_arithmetic_series() {
        assert_eq!(continued(&["1", "2", "3"], 3, false).unwrap(), ["4", "5", "6"]);
        assert_eq!(continued(&["10", "20"], 2, false).unwrap(), ["30", "40"]);
        assert_eq!(continued(&["5", "3"], 2, false).unwrap(), ["1", "-1"]);
        assert_eq!(continued(&["0.1", "0.2", "0.3"], 2, false).unwrap(), ["0.4", "0.5"]);
    }

    #[test]
    fn continues_geometric_series() {
        assert_eq!(continued(&["1", "2", "4"], 2, false).unwrap(), ["8", "16"]);
        assert_eq!(continued(&["3", "9", "27"], 1, false).unwrap(), ["81"]);
        assert_eq!(continued(&["100", "50", "25"], 1, false).unwrap(), ["12.5"]);
    }

    #[test]
    fn two_numbers_are_arithmetic() {
        assert_eq!(continued(&["1", "2"], 2, false).unwrap(), ["3", "4"]);
    }

    #[test]
    fn irregular_numbers_and_zero_ratios_are_copied() {
        assert_eq!(continued(&["1", "2", "5"], 1, false), None);
        assert_eq!(continued(&["0", "1", "3"], 1, false), None);
    }

    #[test]
    fn a_single_number_copies_unless_alternate() {
        assert_eq!(continued(&["7"], 1, false), None);
        assert_eq!(continued(&["7"], 3, true).unwrap(), ["8", "9", "10"]);
    }

    #[test]
    fn alternate_copies_a_series() {
        assert_eq!(continued(&["1", "2", "3"], 1, true), None);
    }

    #[test]
    fn numbered_text_counts() {
        assert_eq!(continued(&["항목1"], 2, false).unwrap(), ["항목2", "항목3"]);
        assert_eq!(continued(&["Item01", "Item03"], 2, false).unwrap(), ["Item05", "Item07"]);
        assert_eq!(continued(&["항목1"], 1, true), None);
    }

    #[test]
    fn plain_and_mismatched_text_is_copied() {
        assert_eq!(continued(&["사과"], 1, false), None);
        assert_eq!(continued(&["a1", "b2"], 1, false), None);
        assert_eq!(continued(&["a1", "a2", "a4"], 1, false), None);
    }

    #[test]
    fn formulas_are_not_series() {
        assert_eq!(continued(&["=1", "=2"], 1, false), None);
    }

    #[test]
    fn positions_before_the_source_run_backwards() {
        let cells: Vec<Cell> = ["1", "2", "4"].iter().map(|text| Cell::parse_input(text).unwrap()).collect();
        let refs: Vec<Option<&Cell>> = cells.iter().map(Some).collect();
        let series = detect(&refs, false).unwrap();
        assert_eq!(series.cell_at(-1).unwrap().value, CellValue::Number(0.5));
        let numbered = Series::Numbered { prefix: "x".into(), first: 1, step: 1, digits: 1 };
        assert_eq!(numbered.cell_at(-1).unwrap().value, CellValue::Text("x0".into()));
        assert_eq!(numbered.cell_at(-2), None, "no negative counts");
    }

    #[test]
    fn values_too_large_fall_back_to_copying() {
        let series = Series::Geometric { first: 1e300, ratio: 1e10 };
        assert_eq!(series.cell_at(5), None);
    }
}
