use crate::{CellError, CellValue, format_date, parse_date};

/// What the user entered into a cell.
#[derive(Debug, Clone, PartialEq)]
pub enum CellInput {
    Literal(CellValue),
    /// Formula text without the leading `=`.
    Formula(String),
}

/// How a number is shown. The value stays a number either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NumberFormat {
    #[default]
    General,
    /// A serial day number shown as `YYYY-MM-DD`.
    Date,
}

/// A stored cell: what was entered, and the value it currently shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub input: CellInput,
    pub value: CellValue,
    /// True when the formula could not be calculated, so `value` is the stored
    /// result from the file rather than a fresh one.
    pub uncalculated: bool,
    /// How a number value is shown. A formula's is set by the engine from what
    /// the formula gives.
    pub format: NumberFormat,
}

impl Cell {
    pub fn literal(value: CellValue) -> Self {
        Self { input: CellInput::Literal(value.clone()), value, uncalculated: false, format: NumberFormat::General }
    }

    /// A date, given as its serial day number.
    pub fn date(serial: f64) -> Self {
        Self { format: NumberFormat::Date, ..Self::literal(CellValue::Number(serial)) }
    }

    /// Builds a formula cell. `formula` has no leading `=`.
    pub fn formula(formula: impl Into<String>, value: CellValue) -> Self {
        Self { input: CellInput::Formula(formula.into()), value, uncalculated: false, format: NumberFormat::General }
    }

    /// Interprets text typed by the user. Returns `None` for blank text.
    pub fn parse_input(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Some(formula) = trimmed.strip_prefix('=').filter(|f| !f.is_empty()) {
            return Some(Self::formula(formula, CellValue::Empty));
        }
        if let Some(number) = parse_number(trimmed) {
            return Some(Self::literal(CellValue::Number(number)));
        }
        if let Some(serial) = parse_date(trimmed) {
            return Some(Self::date(serial));
        }
        if trimmed.eq_ignore_ascii_case("TRUE") {
            return Some(Self::literal(CellValue::Bool(true)));
        }
        if trimmed.eq_ignore_ascii_case("FALSE") {
            return Some(Self::literal(CellValue::Bool(false)));
        }
        if let Some(error) = CellError::from_excel(trimmed) {
            return Some(Self::literal(CellValue::Error(error)));
        }
        Some(Self::literal(CellValue::Text(text.to_owned())))
    }

    /// The text to show when editing this cell. A date shows as a date, so
    /// editing it keeps it one.
    pub fn input_text(&self) -> String {
        match &self.input {
            CellInput::Literal(value) => self.shown(value),
            CellInput::Formula(formula) => format!("={formula}"),
        }
    }

    /// The value as the cell shows it, following its format.
    pub fn display(&self) -> String {
        self.shown(&self.value)
    }

    fn shown(&self, value: &CellValue) -> String {
        match (self.format, value) {
            (NumberFormat::Date, CellValue::Number(n)) => format_date(*n).unwrap_or_else(|| value.to_string()),
            _ => value.to_string(),
        }
    }

    pub fn formula_text(&self) -> Option<&str> {
        match &self.input {
            CellInput::Formula(formula) => Some(formula),
            CellInput::Literal(_) => None,
        }
    }
}

/// Parses a finite decimal number. Rust's `inf` and `nan` spellings are rejected.
fn parse_number(text: &str) -> Option<f64> {
    let looks_numeric = text.bytes().all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'+' | b'e' | b'E'));
    if !looks_numeric {
        return None;
    }
    text.parse::<f64>().ok().filter(|n| n.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value_of(text: &str) -> CellValue {
        Cell::parse_input(text).unwrap().value
    }

    #[test]
    fn parses_input_types() {
        assert_eq!(value_of("3.5"), CellValue::Number(3.5));
        assert_eq!(value_of("true"), CellValue::Bool(true));
        assert_eq!(value_of("#N/A"), CellValue::Error(CellError::Na));
        assert_eq!(value_of("사과"), CellValue::Text("사과".into()));
        assert_eq!(value_of("inf"), CellValue::Text("inf".into()));
        assert_eq!(value_of("nan"), CellValue::Text("nan".into()));
        assert_eq!(value_of("1-2"), CellValue::Text("1-2".into()));
    }

    #[test]
    fn blank_input_clears_the_cell() {
        assert!(Cell::parse_input("").is_none());
        assert!(Cell::parse_input("   ").is_none());
    }

    #[test]
    fn formulas_keep_their_text_for_editing() {
        let cell = Cell::parse_input("=A1+1").unwrap();
        assert_eq!(cell.formula_text(), Some("A1+1"));
        assert_eq!(cell.input_text(), "=A1+1");
    }

    #[test]
    fn typed_dates_become_dates_and_edit_as_dates() {
        let cell = Cell::parse_input("2026/11/2").unwrap();
        assert_eq!(cell.value, CellValue::Number(46_328.0));
        assert_eq!(cell.format, NumberFormat::Date);
        assert_eq!(cell.display(), "2026-11-02");
        assert_eq!(cell.input_text(), "2026-11-02");
        assert_eq!(Cell::parse_input("46328").unwrap().display(), "46328", "a plain number stays one");
        assert_eq!(Cell::date(-5.0).display(), "-5", "a number outside the date range shows as a number");
    }

    #[test]
    fn a_lone_equals_sign_is_text() {
        assert_eq!(value_of("="), CellValue::Text("=".into()));
    }
}
