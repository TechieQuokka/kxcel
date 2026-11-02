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

#[cfg(test)]
mod tests {
    use super::*;

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
