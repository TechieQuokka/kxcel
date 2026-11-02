/// Whether a click during formula entry should insert a cell reference.
///
/// This holds right after an operator, an opening parenthesis, or a comma.
/// Otherwise the formula looks complete, and a click finishes the edit instead.
pub fn accepts_reference(text: &str) -> bool {
    text.starts_with('=')
        && text
            .trim_end()
            .chars()
            .last()
            .is_some_and(|c| matches!(c, '=' | '+' | '-' | '*' | '/' | '^' | '&' | '<' | '>' | '(' | ',' | ':'))
}

/// How many parentheses a formula leaves open, not counting those inside
/// quoted text. Text that is not a formula, or that ends inside quotes, gives
/// 0, since closing parentheses there would not finish it.
pub fn unclosed_parentheses(text: &str) -> usize {
    if !text.starts_with('=') {
        return 0;
    }
    let (mut depth, mut quoted) = (0usize, false);
    for c in text.chars() {
        match c {
            // A doubled quote inside text toggles twice, so it stays text.
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    if quoted { 0 } else { depth }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_open_parentheses_outside_text() {
        for (text, open) in [
            ("=SUM(A1:A3", 1),
            ("=IF(A1>0, SUM(B1:B2", 2),
            ("=SUM(A1)", 0),
            ("=\"(주)\"&LEFT(A1", 1),
            ("=\"a\"\"(\"&(1", 1),
            ("=LEFT(\"abc", 0),
            ("(1+2", 0),
            ("=1)+(2", 1),
        ] {
            assert_eq!(unclosed_parentheses(text), open, "{text}");
        }
    }

    #[test]
    fn accepts_after_operators_and_openers() {
        for text in ["=", "=SUM(", "=A1+", "=IF(A1>", "=SUM(A1, "] {
            assert!(accepts_reference(text), "{text}");
        }
    }

    #[test]
    fn rejects_finished_formulas_and_plain_text() {
        for text in ["=A1", "=SUM(A1)", "=1+2", "hello", "1+", ""] {
            assert!(!accepts_reference(text), "{text}");
        }
    }
}
