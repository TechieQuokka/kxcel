use std::fmt;

/// An error shown in a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellError {
    Div0,
    Value,
    Ref,
    Name,
    Num,
    Na,
    Null,
    /// A circular reference. Excel has no error code for this.
    Cycle,
}

impl CellError {
    /// Parses Excel's error text. `Cycle` has no Excel form, so it is not accepted.
    pub fn from_excel(text: &str) -> Option<Self> {
        Some(match text {
            "#DIV/0!" => Self::Div0,
            "#VALUE!" => Self::Value,
            "#REF!" => Self::Ref,
            "#NAME?" => Self::Name,
            "#NUM!" => Self::Num,
            "#N/A" => Self::Na,
            "#NULL!" => Self::Null,
            _ => return None,
        })
    }
}

impl fmt::Display for CellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Div0 => "#DIV/0!",
            Self::Value => "#VALUE!",
            Self::Ref => "#REF!",
            Self::Name => "#NAME?",
            Self::Num => "#NUM!",
            Self::Na => "#N/A",
            Self::Null => "#NULL!",
            Self::Cycle => "#CYCLE!",
        })
    }
}

/// A calculated or literal cell value.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum CellValue {
    #[default]
    Empty,
    Number(f64),
    Text(String),
    Bool(bool),
    Error(CellError),
}

impl CellValue {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }
}

impl fmt::Display for CellValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => Ok(()),
            Self::Number(n) => f.write_str(&format_number(*n)),
            Self::Text(s) => f.write_str(s),
            Self::Bool(true) => f.write_str("TRUE"),
            Self::Bool(false) => f.write_str("FALSE"),
            Self::Error(e) => write!(f, "{e}"),
        }
    }
}

/// Formats a number with up to 15 significant digits, as Excel's General format does.
///
/// This hides binary rounding noise, so `0.1 + 0.2` displays as `0.3`.
pub fn format_number(n: f64) -> String {
    if n == 0.0 {
        return "0".to_owned();
    }
    if !n.is_finite() {
        return n.to_string();
    }
    let rounded: f64 = format!("{n:.14e}").parse().unwrap_or(n);
    let magnitude = rounded.abs();
    if !(1e-9..1e15).contains(&magnitude) {
        let text = format!("{rounded:e}");
        let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
        let sign = if exponent.starts_with('-') { "" } else { "+" };
        return format!("{mantissa}E{sign}{exponent}");
    }
    format!("{rounded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numbers_like_general_format() {
        for (n, text) in [
            (0.0, "0"),
            (-0.0, "0"),
            (3.0, "3"),
            (-2.5, "-2.5"),
            (0.1 + 0.2, "0.3"),
            (1234567.0, "1234567"),
            (1e15, "1E+15"),
            (1.5e-10, "1.5E-10"),
        ] {
            assert_eq!(format_number(n), text);
        }
    }

    #[test]
    fn excel_error_text_round_trips() {
        for error in [
            CellError::Div0,
            CellError::Value,
            CellError::Ref,
            CellError::Name,
            CellError::Num,
            CellError::Na,
            CellError::Null,
        ] {
            assert_eq!(CellError::from_excel(&error.to_string()), Some(error));
        }
        assert_eq!(CellError::from_excel("#CYCLE!"), None);
    }
}
