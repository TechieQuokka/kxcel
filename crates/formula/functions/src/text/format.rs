//! The number and date formats `TEXT` shows values in, and reading numbers
//! back for `VALUE`.

use excel_model::{CellError, parse_date, serial_date};

/// One piece of a format: text shown as it is, or a placeholder.
#[derive(Debug, Clone, PartialEq)]
enum Piece {
    Literal(String),
    /// A run of one placeholder letter, such as `yyyy` or `0`.
    Code(char, usize),
}

/// Splits a format into literal text and runs of placeholder letters. Text in
/// double quotes, and a character after `\`, are literal.
fn pieces(format: &str) -> Vec<Piece> {
    let mut pieces: Vec<Piece> = Vec::new();
    let push_literal = |pieces: &mut Vec<Piece>, c: char| match pieces.last_mut() {
        Some(Piece::Literal(text)) => text.push(c),
        _ => pieces.push(Piece::Literal(c.to_string())),
    };
    let mut chars = format.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                for inner in chars.by_ref() {
                    if inner == '"' {
                        break;
                    }
                    push_literal(&mut pieces, inner);
                }
            }
            '\\' => {
                if let Some(next) = chars.next() {
                    push_literal(&mut pieces, next);
                }
            }
            c if "0#,.yYmMdD".contains(c) => {
                let c = c.to_ascii_lowercase();
                match pieces.last_mut() {
                    Some(Piece::Code(code, count)) if *code == c => *count += 1,
                    _ => pieces.push(Piece::Code(c, 1)),
                }
            }
            c => push_literal(&mut pieces, c),
        }
    }
    pieces
}

/// Shows `value` in `format`. A format with a year or day shows a date, and
/// any other shows a number.
pub(crate) fn apply(value: f64, format: &str) -> Result<String, CellError> {
    let pieces = pieces(format);
    let is_date = pieces.iter().any(|piece| matches!(piece, Piece::Code('y' | 'd', _)));
    if is_date { date(value, &pieces) } else { Ok(number(value, &pieces)) }
}

fn date(value: f64, pieces: &[Piece]) -> Result<String, CellError> {
    let (year, month, day) = serial_date(value).ok_or(CellError::Value)?;
    let mut text = String::new();
    for piece in pieces {
        match piece {
            Piece::Literal(literal) => text.push_str(literal),
            Piece::Code('y', count) if *count <= 2 => text.push_str(&format!("{:02}", year % 100)),
            Piece::Code('y', _) => text.push_str(&format!("{year:04}")),
            Piece::Code('m', 1) => text.push_str(&month.to_string()),
            Piece::Code('m', _) => text.push_str(&format!("{month:02}")),
            Piece::Code('d', 1) => text.push_str(&day.to_string()),
            Piece::Code('d', _) => text.push_str(&format!("{day:02}")),
            Piece::Code(code, count) => text.extend(std::iter::repeat_n(*code, *count)),
        }
    }
    Ok(text)
}

/// Shows a number. The placeholders run from the first `0` or `#` to the last
/// placeholder: `0` is a digit always shown, `#` one shown only when needed,
/// a comma in the whole part groups thousands, and the period starts the
/// decimals. Literal text before and after is kept, and a `%` there shows the
/// number as a percentage.
fn number(value: f64, pieces: &[Piece]) -> String {
    let is_digit = |piece: &Piece| matches!(piece, Piece::Code('0' | '#', _));
    let Some(first) = pieces.iter().position(is_digit) else {
        // No digits to show: the format is all literal text.
        return pieces.iter().map(literal_text).collect();
    };
    let last = pieces.iter().rposition(|piece| matches!(piece, Piece::Code('0' | '#' | ',' | '.', _))).unwrap_or(first);
    // A period just before the first digit belongs to the number too.
    let first = if first > 0 && matches!(pieces[first - 1], Piece::Code('.', _)) { first - 1 } else { first };
    let (before, pattern, after) = (&pieces[..first], &pieces[first..=last], &pieces[last + 1..]);
    let percent = before.iter().chain(after).any(|piece| literal_text(piece).contains('%'));

    let mut whole_zeros = 0;
    let (mut decimal_zeros, mut decimal_places) = (0, 0);
    let (mut grouped, mut in_decimals) = (false, false);
    for piece in pattern {
        match piece {
            Piece::Code('.', _) => in_decimals = true,
            Piece::Code(',', _) if !in_decimals => grouped = true,
            Piece::Code('0', count) if in_decimals => {
                decimal_zeros += count;
                decimal_places += count;
            }
            Piece::Code('#', count) if in_decimals => decimal_places += count,
            Piece::Code('0', count) => whole_zeros += count,
            _ => {}
        }
    }

    let value = if percent { value * 100.0 } else { value };
    let rounded = round_half_away(value.abs(), decimal_places);
    let fixed = format!("{rounded:.decimal_places$}");
    let (whole, decimals) = fixed.split_once('.').unwrap_or((&fixed, ""));
    let decimals = decimals.trim_end_matches('0');
    let decimals = format!("{decimals:0<decimal_zeros$}");
    let whole = whole.trim_start_matches('0');
    let whole = format!("{whole:0>whole_zeros$}");
    let whole = if grouped { group_thousands(&whole) } else { whole };

    let mut text = String::new();
    if value < 0.0 && rounded != 0.0 {
        text.push('-');
    }
    text.extend(before.iter().map(literal_text));
    text.push_str(&whole);
    if !decimals.is_empty() {
        text.push('.');
        text.push_str(&decimals);
    }
    text.extend(after.iter().map(literal_text));
    text
}

fn literal_text(piece: &Piece) -> String {
    match piece {
        Piece::Literal(text) => text.clone(),
        Piece::Code(code, count) => std::iter::repeat_n(*code, *count).collect(),
    }
}

/// Rounds half away from zero, after cutting to 15 significant digits as Excel does.
fn round_half_away(value: f64, places: usize) -> f64 {
    let scale = 10f64.powi(places.min(15) as i32);
    let scaled: f64 = format!("{:.14e}", value * scale).parse().unwrap_or(value * scale);
    scaled.round() / scale
}

fn group_thousands(digits: &str) -> String {
    let mut grouped = String::new();
    for (index, c) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(c);
    }
    grouped
}

/// Reads a number from text, allowing commas between thousands, a trailing
/// `%`, and dates such as `2026-11-02`.
pub(crate) fn parse_number(text: &str) -> Option<f64> {
    let text = text.trim();
    if let Some(serial) = parse_date(text) {
        return Some(serial);
    }
    let (digits, scale) = match text.strip_suffix('%') {
        Some(rest) => (rest.trim_end(), 0.01),
        None => (text, 1.0),
    };
    let digits = digits.replace(',', "");
    let plain = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit() || b"+-.eE".contains(&b));
    if !plain {
        return None;
    }
    digits.parse::<f64>().ok().filter(|n| n.is_finite()).map(|n| n * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        for (value, format, text) in [
            (1234.5, "#,##0.00", "1,234.50"),
            (1234.5, "0", "1235"),
            (0.5, "0", "1"),
            (2.675, "0.00", "2.68"),
            (-1234.5, "#,##0", "-1,235"),
            (0.256, "0.0%", "25.6%"),
            (0.25, "0%", "25%"),
            (7.0, "000", "007"),
            (1.5, "#.##", "1.5"),
            (0.5, "#.00", ".50"),
            (1234567.0, "#,##0", "1,234,567"),
            (12.0, "0\"개\"", "12개"),
            (3.0, "\"합계: \"0", "합계: 3"),
            (-0.001, "0.00", "0.00"),
            (5.0, "없음", "없음"),
        ] {
            assert_eq!(apply(value, format).unwrap(), text, "{value} as {format}");
        }
    }

    #[test]
    fn dates() {
        for (format, text) in [
            ("yyyy-mm-dd", "2026-11-02"),
            ("yyyy/m/d", "2026/11/2"),
            ("yy.mm.dd", "26.11.02"),
            ("yyyy년 m월 d일", "2026년 11월 2일"),
            ("mm/dd", "11/02"),
        ] {
            assert_eq!(apply(46_328.0, format).unwrap(), text, "{format}");
        }
        assert_eq!(apply(-1.0, "yyyy-mm-dd"), Err(CellError::Value));
    }

    #[test]
    fn reads_numbers_back() {
        assert_eq!(parse_number(" 1,234.5 "), Some(1234.5));
        assert_eq!(parse_number("15%"), Some(0.15));
        assert_eq!(parse_number("2026-11-02"), Some(46_328.0));
        assert_eq!(parse_number("-3e2"), Some(-300.0));
        for text in ["", "abc", "1.2.3x", "inf", "%"] {
            assert_eq!(parse_number(text), None, "{text}");
        }
    }
}
