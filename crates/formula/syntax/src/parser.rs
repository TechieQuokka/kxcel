use std::fmt;

use crate::ast::{PERCENT_PRECEDENCE, PREFIX_PRECEDENCE};
use crate::{BinaryOp, CellRef, Expr, Token, UnaryOp, tokenize};

/// Why formula text could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// Parses formula text. The text has no leading `=`.
pub fn parse(source: &str) -> Result<Expr, ParseError> {
    let tokens = tokenize(source)?;
    let mut parser = Parser { tokens, position: 0 };
    let expr = parser.expression(0)?;
    match parser.peek() {
        None => Ok(expr),
        Some(token) => Err(ParseError(format!("Unexpected {token:?}"))),
    }
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).cloned();
        self.position += 1;
        token
    }

    fn eat(&mut self, token: &Token) -> bool {
        let found = self.peek() == Some(token);
        if found {
            self.position += 1;
        }
        found
    }

    /// Precedence climbing. Reads operators that bind at least as tightly as `min`.
    fn expression(&mut self, min: u8) -> Result<Expr, ParseError> {
        let mut left = self.prefix()?;
        loop {
            if self.peek() == Some(&Token::Percent) && PERCENT_PRECEDENCE >= min {
                self.position += 1;
                left = Expr::Unary(UnaryOp::Percent, Box::new(left));
                continue;
            }
            let Some(op) = self.peek().and_then(binary_op) else {
                break;
            };
            if op.precedence() < min {
                break;
            }
            self.position += 1;
            let right = self.expression(op.precedence() + 1)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn prefix(&mut self) -> Result<Expr, ParseError> {
        if self.eat(&Token::Minus) {
            let inner = self.expression(PREFIX_PRECEDENCE)?;
            return Ok(Expr::Unary(UnaryOp::Neg, Box::new(inner)));
        }
        if self.eat(&Token::Plus) {
            let inner = self.expression(PREFIX_PRECEDENCE)?;
            return Ok(Expr::Unary(UnaryOp::Plus, Box::new(inner)));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        match self.next() {
            Some(Token::Number(n)) => Ok(Expr::Number(n)),
            Some(Token::Text(text)) => Ok(Expr::Text(text)),
            Some(Token::Error(text)) => Ok(Expr::Error(text)),
            Some(Token::LeftParen) => {
                let inner = self.expression(0)?;
                if self.eat(&Token::RightParen) {
                    Ok(inner)
                } else {
                    Err(ParseError("Missing closing parenthesis".to_owned()))
                }
            }
            Some(Token::QuotedName(name)) => {
                self.expect_bang()?;
                self.reference(Some(name))
            }
            Some(Token::Name(name)) => self.named(name),
            Some(token) => Err(ParseError(format!("Unexpected {token:?}"))),
            None => Err(ParseError("Unexpected end of formula".to_owned())),
        }
    }

    fn expect_bang(&mut self) -> Result<(), ParseError> {
        if self.eat(&Token::Bang) { Ok(()) } else { Err(ParseError("Expected ! after sheet name".to_owned())) }
    }

    fn named(&mut self, name: String) -> Result<Expr, ParseError> {
        if self.eat(&Token::Bang) {
            return self.reference(Some(name));
        }
        if self.eat(&Token::LeftParen) {
            let args = self.arguments()?;
            return Ok(Expr::Call { name: name.to_ascii_uppercase(), args });
        }
        if name.eq_ignore_ascii_case("TRUE") {
            return Ok(Expr::Bool(true));
        }
        if name.eq_ignore_ascii_case("FALSE") {
            return Ok(Expr::Bool(false));
        }
        self.position -= 1;
        self.reference(None)
    }

    /// Reads `A1` or `A1:B2`, optionally after a sheet name.
    fn reference(&mut self, sheet: Option<String>) -> Result<Expr, ParseError> {
        let start = self.cell_ref()?;
        if !self.eat(&Token::Colon) {
            return Ok(Expr::Cell { sheet, cell: start });
        }
        let end = self.cell_ref()?;
        Ok(Expr::Range { sheet, start, end })
    }

    fn cell_ref(&mut self) -> Result<CellRef, ParseError> {
        match self.next() {
            Some(Token::Name(name)) => CellRef::parse(&name).ok_or_else(|| ParseError(format!("Unknown name: {name}"))),
            Some(token) => Err(ParseError(format!("Expected a cell reference, found {token:?}"))),
            None => Err(ParseError("Expected a cell reference".to_owned())),
        }
    }

    fn arguments(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        if self.eat(&Token::RightParen) {
            return Ok(args);
        }
        loop {
            args.push(self.expression(0)?);
            if self.eat(&Token::RightParen) {
                return Ok(args);
            }
            if !self.eat(&Token::Comma) {
                return Err(ParseError("Expected , or ) in function call".to_owned()));
            }
        }
    }
}

fn binary_op(token: &Token) -> Option<BinaryOp> {
    Some(match token {
        Token::Plus => BinaryOp::Add,
        Token::Minus => BinaryOp::Sub,
        Token::Star => BinaryOp::Mul,
        Token::Slash => BinaryOp::Div,
        Token::Caret => BinaryOp::Pow,
        Token::Ampersand => BinaryOp::Concat,
        Token::Equal => BinaryOp::Eq,
        Token::NotEqual => BinaryOp::Ne,
        Token::Less => BinaryOp::Lt,
        Token::LessEqual => BinaryOp::Le,
        Token::Greater => BinaryOp::Gt,
        Token::GreaterEqual => BinaryOp::Ge,
        _ => return None,
    })
}
