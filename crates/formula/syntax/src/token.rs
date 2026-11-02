/// A lexical token of a formula.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    /// A double-quoted string, with `""` already unescaped.
    Text(String),
    /// A name: a function, a cell reference, `TRUE`, `FALSE`, or a sheet name.
    Name(String),
    /// A single-quoted sheet name, with `''` already unescaped.
    QuotedName(String),
    /// An error literal, such as `#REF!`.
    Error(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Ampersand,
    Percent,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    LeftParen,
    RightParen,
    Comma,
    Colon,
    Bang,
}
