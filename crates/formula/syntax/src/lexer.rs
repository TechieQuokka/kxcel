use std::iter::Peekable;
use std::str::Chars;

use crate::{ParseError, Token};

/// Splits formula text into tokens. The text has no leading `=`.
pub fn tokenize(source: &str) -> Result<Vec<Token>, ParseError> {
    let mut chars = source.chars().peekable();
    let mut tokens = Vec::new();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        let token = match c {
            '0'..='9' | '.' => number(&mut chars)?,
            '"' => Token::Text(quoted(&mut chars, '"')?),
            '\'' => Token::QuotedName(quoted(&mut chars, '\'')?),
            '#' => error_literal(&mut chars),
            '<' | '>' => comparison(&mut chars),
            c if is_name_start(c) => Token::Name(take_while(&mut chars, is_name_part)),
            _ => {
                chars.next();
                single(c).ok_or_else(|| ParseError(format!("Unexpected character: {c}")))?
            }
        };
        tokens.push(token);
    }
    Ok(tokens)
}

fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || matches!(c, '_' | '$')
}

fn is_name_part(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '$' | '.')
}

fn single(c: char) -> Option<Token> {
    Some(match c {
        '+' => Token::Plus,
        '-' => Token::Minus,
        '*' => Token::Star,
        '/' => Token::Slash,
        '^' => Token::Caret,
        '&' => Token::Ampersand,
        '%' => Token::Percent,
        '=' => Token::Equal,
        '(' => Token::LeftParen,
        ')' => Token::RightParen,
        ',' => Token::Comma,
        ':' => Token::Colon,
        '!' => Token::Bang,
        _ => return None,
    })
}

fn take_while(chars: &mut Peekable<Chars<'_>>, keep: impl Fn(char) -> bool) -> String {
    let mut text = String::new();
    while let Some(&c) = chars.peek() {
        if !keep(c) {
            break;
        }
        text.push(c);
        chars.next();
    }
    text
}

fn number(chars: &mut Peekable<Chars<'_>>) -> Result<Token, ParseError> {
    let mut text = take_while(chars, |c| c.is_ascii_digit() || c == '.');
    if matches!(chars.peek(), Some('e' | 'E')) {
        let mut lookahead = chars.clone();
        lookahead.next();
        if matches!(lookahead.peek(), Some('+' | '-')) {
            lookahead.next();
        }
        if lookahead.peek().is_some_and(char::is_ascii_digit) {
            text.push('e');
            chars.next();
            if let Some(sign) = chars.next_if(|c| matches!(c, '+' | '-')) {
                text.push(sign);
            }
            text.push_str(&take_while(chars, |c| c.is_ascii_digit()));
        }
    }
    text.parse().map(Token::Number).map_err(|_| ParseError(format!("Invalid number: {text}")))
}

/// Reads text between quotes, where a doubled quote stands for one quote.
fn quoted(chars: &mut Peekable<Chars<'_>>, quote: char) -> Result<String, ParseError> {
    chars.next();
    let mut text = String::new();
    while let Some(c) = chars.next() {
        if c != quote {
            text.push(c);
        } else if chars.next_if_eq(&quote).is_some() {
            text.push(quote);
        } else {
            return Ok(text);
        }
    }
    Err(ParseError("Unclosed quote".to_owned()))
}

fn error_literal(chars: &mut Peekable<Chars<'_>>) -> Token {
    let text = take_while(chars, |c| c.is_alphanumeric() || matches!(c, '#' | '/' | '!' | '?'));
    Token::Error(text)
}

fn comparison(chars: &mut Peekable<Chars<'_>>) -> Token {
    let first = chars.next();
    match (first, chars.peek()) {
        (Some('<'), Some('=')) => {
            chars.next();
            Token::LessEqual
        }
        (Some('<'), Some('>')) => {
            chars.next();
            Token::NotEqual
        }
        (Some('>'), Some('=')) => {
            chars.next();
            Token::GreaterEqual
        }
        (Some('<'), _) => Token::Less,
        _ => Token::Greater,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_operators_and_names() {
        assert_eq!(
            tokenize("SUM($A$1:B2)>=1").unwrap(),
            [
                Token::Name("SUM".into()),
                Token::LeftParen,
                Token::Name("$A$1".into()),
                Token::Colon,
                Token::Name("B2".into()),
                Token::RightParen,
                Token::GreaterEqual,
                Token::Number(1.0),
            ]
        );
    }

    #[test]
    fn reads_numbers() {
        assert_eq!(tokenize("1.5e3").unwrap(), [Token::Number(1500.0)]);
        assert_eq!(tokenize(".5").unwrap(), [Token::Number(0.5)]);
        assert_eq!(tokenize("2e").unwrap(), [Token::Number(2.0), Token::Name("e".into())]);
        assert!(tokenize("1.2.3").is_err());
    }

    #[test]
    fn reads_quoted_text() {
        assert_eq!(tokenize(r#""a""b""#).unwrap(), [Token::Text("a\"b".into())]);
        assert_eq!(tokenize("'My ''Sheet'").unwrap(), [Token::QuotedName("My 'Sheet".into())]);
        assert!(tokenize("\"open").is_err());
    }

    #[test]
    fn reads_korean_names_and_text() {
        assert_eq!(tokenize("\"사과\"").unwrap(), [Token::Text("사과".into())]);
        assert_eq!(tokenize("시트1").unwrap(), [Token::Name("시트1".into())]);
    }

    #[test]
    fn rejects_unknown_characters() {
        assert!(tokenize("1 ~ 2").is_err());
    }

    #[test]
    fn reads_error_literals() {
        assert_eq!(tokenize("#DIV/0!").unwrap(), [Token::Error("#DIV/0!".into())]);
        assert_eq!(tokenize("#N/A").unwrap(), [Token::Error("#N/A".into())]);
    }
}
