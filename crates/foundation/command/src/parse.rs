/// Splits a command line on whitespace. Double quotes keep spaces in an argument.
///
/// An unclosed quote runs to the end of the line.
pub fn split_line(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_part = false;
    let mut quoted = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                in_part = true;
            }
            c if c.is_whitespace() && !quoted => {
                if in_part {
                    parts.push(std::mem::take(&mut current));
                    in_part = false;
                }
            }
            c => {
                current.push(c);
                in_part = true;
            }
        }
    }
    if in_part {
        parts.push(current);
    }
    parts
}

/// Writes an argument so that [`split_line`] reads it back unchanged: quoted if
/// it is empty or holds whitespace. Quotes inside it cannot be kept.
pub fn quote_arg(arg: &str) -> String {
    if arg.is_empty() || arg.contains(char::is_whitespace) { format!("\"{arg}\"") } else { arg.to_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_arguments_split_back_unchanged() {
        for arg in ["data.xlsx", "my file.xlsx", "", "사과 배"] {
            assert_eq!(split_line(&format!("open {}", quote_arg(arg))), ["open", arg]);
        }
    }

    #[test]
    fn splits_on_whitespace() {
        assert_eq!(split_line("  sort  B desc "), ["sort", "B", "desc"]);
        assert!(split_line("   ").is_empty());
    }

    #[test]
    fn quotes_keep_spaces() {
        assert_eq!(split_line(r#"open "my file.xlsx""#), ["open", "my file.xlsx"]);
        assert_eq!(split_line(r#"find """#), ["find", ""]);
        assert_eq!(split_line(r#"find "a b"#), ["find", "a b"]);
    }

    #[test]
    fn keeps_korean_text() {
        assert_eq!(split_line("find 사과"), ["find", "사과"]);
    }
}
