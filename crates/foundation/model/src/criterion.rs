//! Conditions such as `">5"`, `"apple"`, or `"a*"`, as `COUNTIF` reads them.
//! Used by the functions like `SUMIF` and by table filters.

use crate::{CellError, CellValue, parse_date};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq)]
enum Operand {
    Number(f64),
    Bool(bool),
    /// Lowercase text, which may hold the wildcards `*` and `?`.
    Text(String),
    Error(CellError),
    /// Nothing after the operator: blank cells.
    Blank,
}

/// A condition a cell either meets or not, as Excel reads one.
#[derive(Debug, Clone, PartialEq)]
pub struct Criterion {
    op: Op,
    operand: Operand,
}

impl Criterion {
    /// Reads a condition. Text may start with `=`, `<>`, `<`, `<=`, `>`, or `>=`,
    /// and what follows is a number, a date, `TRUE` or `FALSE`, or text. A plain
    /// value means equal to it.
    pub fn new(value: &CellValue) -> Self {
        let equal = |operand| Self { op: Op::Eq, operand };
        match value {
            CellValue::Number(n) => equal(Operand::Number(*n)),
            CellValue::Bool(b) => equal(Operand::Bool(*b)),
            CellValue::Error(error) => equal(Operand::Error(*error)),
            CellValue::Empty => equal(Operand::Blank),
            CellValue::Text(text) => {
                let (op, rest) =
                    [("<=", Op::Le), (">=", Op::Ge), ("<>", Op::Ne), ("<", Op::Lt), (">", Op::Gt), ("=", Op::Eq)]
                        .into_iter()
                        .find_map(|(prefix, op)| text.strip_prefix(prefix).map(|rest| (op, rest)))
                        .unwrap_or((Op::Eq, text.as_str()));
                Self { op, operand: operand(rest) }
            }
        }
    }

    /// Whether a cell's value meets the condition.
    pub fn matches(&self, value: &CellValue) -> bool {
        let blank = matches!(value, CellValue::Empty) || matches!(value, CellValue::Text(t) if t.is_empty());
        let equal = match (&self.operand, value) {
            (Operand::Blank, _) => blank,
            (Operand::Number(n), CellValue::Number(m)) => n == m,
            // As in Excel, text that reads as the number counts as equal to it.
            (Operand::Number(n), CellValue::Text(text)) => text.trim().parse::<f64>().is_ok_and(|m| m == *n),
            (Operand::Bool(b), CellValue::Bool(c)) => b == c,
            (Operand::Text(pattern), CellValue::Text(text)) => wildcard(pattern, &text.to_lowercase()),
            (Operand::Error(e), CellValue::Error(f)) => e == f,
            _ => false,
        };
        let ordering = match (&self.operand, value) {
            (Operand::Number(n), CellValue::Number(m)) => m.partial_cmp(n),
            (Operand::Text(pattern), CellValue::Text(text)) => Some(text.to_lowercase().cmp(pattern)),
            _ => None,
        };
        match self.op {
            Op::Eq => equal,
            Op::Ne => !equal,
            Op::Lt => ordering.is_some_and(|o| o.is_lt()),
            Op::Le => ordering.is_some_and(|o| o.is_le()),
            Op::Gt => ordering.is_some_and(|o| o.is_gt()),
            Op::Ge => ordering.is_some_and(|o| o.is_ge()),
        }
    }
}

fn operand(text: &str) -> Operand {
    if text.is_empty() {
        return Operand::Blank;
    }
    if let Ok(n) = text.trim().parse::<f64>()
        && n.is_finite()
    {
        return Operand::Number(n);
    }
    if let Some(serial) = parse_date(text.trim()) {
        return Operand::Number(serial);
    }
    if text.eq_ignore_ascii_case("TRUE") {
        return Operand::Bool(true);
    }
    if text.eq_ignore_ascii_case("FALSE") {
        return Operand::Bool(false);
    }
    match CellError::from_excel(text) {
        Some(error) => Operand::Error(error),
        None => Operand::Text(text.to_lowercase()),
    }
}

/// Matches text against a pattern where `*` is any run of characters, `?` is
/// one character, and `~` makes the next character plain.
fn wildcard(pattern: &str, text: &str) -> bool {
    #[derive(Clone, Copy, PartialEq)]
    enum Part {
        Char(char),
        One,
        Any,
    }
    let mut parts = Vec::new();
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        parts.push(match c {
            '~' => Part::Char(chars.next().unwrap_or('~')),
            '?' => Part::One,
            '*' => Part::Any,
            c => Part::Char(c),
        });
    }
    let text: Vec<char> = text.chars().collect();
    // matched[j]: whether the parts so far match the first j characters.
    let mut matched = vec![false; text.len() + 1];
    matched[0] = true;
    for part in parts {
        let mut next = vec![false; text.len() + 1];
        for j in 0..=text.len() {
            next[j] = match part {
                Part::Any => matched[j] || (j > 0 && next[j - 1]),
                Part::One => j > 0 && matched[j - 1],
                Part::Char(c) => j > 0 && matched[j - 1] && text[j - 1] == c,
            };
        }
        matched = next;
    }
    matched[text.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meets(criterion: &str, value: CellValue) -> bool {
        Criterion::new(&CellValue::Text(criterion.into())).matches(&value)
    }

    fn n(value: f64) -> CellValue {
        CellValue::Number(value)
    }

    fn t(text: &str) -> CellValue {
        CellValue::Text(text.into())
    }

    #[test]
    fn compares_numbers() {
        assert!(meets(">5", n(6.0)) && !meets(">5", n(5.0)));
        assert!(meets(">=5", n(5.0)) && meets("<=5", n(5.0)) && meets("<5", n(-1.0)));
        assert!(meets("5", n(5.0)) && meets("=5", t("5")), "text that reads as the number is equal");
        assert!(!meets(">5", t("9")), "ordering needs a number");
        assert!(meets("<>5", n(4.0)) && meets("<>5", CellValue::Empty));
    }

    #[test]
    fn matches_text_ignoring_case_with_wildcards() {
        assert!(meets("apple", t("Apple")));
        assert!(meets("a*", t("avocado")) && !meets("a*", t("banana")));
        assert!(meets("?pple", t("Apple")) && !meets("?pple", t("pple")));
        assert!(meets("*사과*", t("빨간 사과 한 개")));
        assert!(meets("~*", t("*")) && !meets("~*", t("x")));
        assert!(meets("<>apple", t("pear")) && !meets("<>apple", t("APPLE")));
        assert!(meets(">b", t("cat")) && !meets(">b", t("a")));
    }

    #[test]
    fn blank_conditions() {
        assert!(meets("", CellValue::Empty) && meets("", t("")) && !meets("", n(0.0)));
        assert!(meets("=", CellValue::Empty));
        assert!(meets("<>", n(0.0)) && !meets("<>", CellValue::Empty));
    }

    #[test]
    fn other_values() {
        assert!(Criterion::new(&n(3.0)).matches(&n(3.0)));
        assert!(Criterion::new(&CellValue::Bool(true)).matches(&CellValue::Bool(true)));
        assert!(meets("TRUE", CellValue::Bool(true)) && !meets("TRUE", t("yes")));
        assert!(meets("#N/A", CellValue::Error(CellError::Na)));
        assert!(meets(">=2026-01-01", n(46_100.0)), "dates compare as their numbers");
    }
}
