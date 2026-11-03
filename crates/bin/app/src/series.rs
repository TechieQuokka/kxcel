//! Detects a series in the cells a fill starts from, so a fill can continue it.

use excel_model::{Cell, CellInput, CellValue};

/// How a fill continues a line of cells.
#[derive(Debug, Clone, PartialEq)]
pub enum Series {
    /// Numbers with a constant difference: 1, 2, 3.
    Arithmetic { first: f64, step: f64 },
    /// Numbers with a constant ratio: 1, 2, 4.
    Geometric { first: f64, ratio: f64 },
    /// Text with numbers in it, such as `사1과10`. Every cell has the same
    /// text around the numbers, `parts`, one more than there are numbers, and
    /// each number has its own way of going on.
    Numbered { parts: Vec<String>, numbers: Vec<Slot> },
}

/// How one number inside a text goes on.
#[derive(Debug, Clone, PartialEq)]
pub enum Slot {
    /// A constant difference: 1, 2, 3. `digits` keeps zero padding, as in `07`.
    Counting { first: i64, step: i64, digits: usize },
    /// No pattern, so the source numbers repeat in order.
    Repeating(Vec<(i64, usize)>),
}

impl Slot {
    fn at(&self, n: i64) -> Option<String> {
        let (value, digits) = match self {
            Self::Counting { first, step, digits } => (first.checked_add(step.checked_mul(n)?)?, *digits),
            Self::Repeating(values) => values[n.rem_euclid(values.len() as i64) as usize],
        };
        (value >= 0).then(|| format!("{value:0digits$}"))
    }
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
            Self::Numbered { parts, numbers } => {
                let mut text = parts[0].clone();
                for (slot, after) in numbers.iter().zip(&parts[1..]) {
                    text.push_str(&slot.at(n)?);
                    text.push_str(after);
                }
                Some(Cell::literal(CellValue::Text(text)))
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

/// Text cut around its numbers: the text between them, and each number with
/// its digit count.
type Split<'a> = (Vec<&'a str>, Vec<(i64, usize)>);

/// Splits text such as `사1과07` into the text around its numbers, `사`, `과`,
/// and an empty end, and the numbers with their digit counts, 1 and 7 with 2.
/// `None` for text without numbers or without anything else.
fn split_numbers(text: &str) -> Option<Split<'_>> {
    let mut parts = Vec::new();
    let mut numbers = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(|c: char| c.is_ascii_digit()) {
        let digits = rest[start..].find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len() - start);
        parts.push(&rest[..start]);
        let number = &rest[start..start + digits];
        numbers.push((number.parse().ok()?, number.len()));
        rest = &rest[start + digits..];
    }
    parts.push(rest);
    let has_text = parts.iter().any(|part| !part.is_empty());
    (has_text && !numbers.is_empty()).then_some((parts, numbers))
}

/// Text cells with the same text around their numbers. With several cells,
/// each number counts on if it changes by a constant step, and repeats its
/// values if not. A single cell counts up its last number, as in Excel.
fn detect_numbered_text(cells: &[Option<&Cell>], alternate: bool) -> Option<Series> {
    let split: Vec<Split<'_>> = cells
        .iter()
        .map(|cell| match &(*cell)?.input {
            CellInput::Literal(CellValue::Text(text)) => split_numbers(text),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let parts = &split[0].0;
    if split.iter().any(|(other, _)| other != parts) {
        return None;
    }
    let count = split[0].1.len();
    let numbers: Vec<Slot> = if split.len() == 1 {
        if alternate {
            return None;
        }
        let values = &split[0].1;
        (0..count)
            .map(|i| {
                let (first, digits) = values[i];
                if i + 1 == count {
                    Slot::Counting { first, step: 1, digits }
                } else {
                    Slot::Repeating(vec![values[i]])
                }
            })
            .collect()
    } else {
        (0..count)
            .map(|i| {
                let column: Vec<(i64, usize)> = split.iter().map(|(_, numbers)| numbers[i]).collect();
                let step = column[1].0 - column[0].0;
                if column.windows(2).all(|pair| pair[1].0 - pair[0].0 == step) {
                    Slot::Counting { first: column[0].0, step, digits: column[0].1 }
                } else {
                    Slot::Repeating(column)
                }
            })
            .collect()
    };
    // With nothing counting, every number repeats, which is a plain copy.
    numbers
        .iter()
        .any(|slot| matches!(slot, Slot::Counting { .. }))
        .then(|| Series::Numbered { parts: parts.iter().map(|part| (*part).to_owned()).collect(), numbers })
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
    fn numbers_anywhere_in_text_count() {
        assert_eq!(continued(&["사1과", "사2과"], 2, false).unwrap(), ["사3과", "사4과"]);
        assert_eq!(continued(&["1사과", "2사과", "3사과"], 1, false).unwrap(), ["4사과"]);
        assert_eq!(continued(&["100원", "200원"], 2, false).unwrap(), ["300원", "400원"]);
        assert_eq!(continued(&["1사과"], 2, false).unwrap(), ["2사과", "3사과"]);
    }

    #[test]
    fn each_number_in_text_has_its_own_series() {
        assert_eq!(continued(&["사1과10", "사2과20", "사3과30"], 2, false).unwrap(), ["사4과40", "사5과50"]);
        assert_eq!(
            continued(&["사1과10", "사2과7", "사3과42"], 3, false).unwrap(),
            ["사4과10", "사5과7", "사6과42"],
            "a number without a pattern repeats while the other counts"
        );
        assert_eq!(
            continued(&["2026년 1월"], 2, false).unwrap(),
            ["2026년 2월", "2026년 3월"],
            "one cell counts its last number"
        );
        assert_eq!(continued(&["A07-1", "A08-1"], 1, false).unwrap(), ["A09-1"]);
    }

    #[test]
    fn plain_and_mismatched_text_is_copied() {
        assert_eq!(continued(&["사과"], 1, false), None);
        assert_eq!(continued(&["a1", "b2"], 1, false), None);
        assert_eq!(continued(&["a1", "a2", "a4"], 1, false), None);
        assert_eq!(continued(&["사1과", "배2과"], 1, false), None, "different text around the numbers");
        assert_eq!(continued(&["x1y5", "x3y2", "x2y9"], 1, false), None, "nothing counts, so it copies");
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
        let numbered = Series::Numbered {
            parts: vec!["x".into(), String::new()],
            numbers: vec![Slot::Counting { first: 1, step: 1, digits: 1 }],
        };
        assert_eq!(numbered.cell_at(-1).unwrap().value, CellValue::Text("x0".into()));
        assert_eq!(numbered.cell_at(-2), None, "no negative counts");
    }

    #[test]
    fn values_too_large_fall_back_to_copying() {
        let series = Series::Geometric { first: 1e300, ratio: 1e10 };
        assert_eq!(series.cell_at(5), None);
    }
}
