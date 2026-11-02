use std::fmt;

use excel_model::{CellAddr, col_name};

/// A cell reference with its `$` markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRef {
    pub addr: CellAddr,
    pub abs_row: bool,
    pub abs_col: bool,
}

impl CellRef {
    /// Parses text such as `B2` or `$B$2`. Returns `None` for anything else.
    pub fn parse(text: &str) -> Option<Self> {
        let (abs_col, rest) = strip_dollar(text);
        let split = rest.find(|c: char| !c.is_ascii_alphabetic())?;
        let (letters, rest) = rest.split_at(split);
        let (abs_row, digits) = strip_dollar(rest);
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let addr = format!("{letters}{digits}").parse().ok()?;
        Some(Self { addr, abs_row, abs_col })
    }
}

fn strip_dollar(text: &str) -> (bool, &str) {
    match text.strip_prefix('$') {
        Some(rest) => (true, rest),
        None => (false, text),
    }
}

impl fmt::Display for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let col_mark = if self.abs_col { "$" } else { "" };
        let row_mark = if self.abs_row { "$" } else { "" };
        write!(f, "{col_mark}{}{row_mark}{}", col_name(self.addr.col), self.addr.row + 1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Plus,
    Percent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Concat,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl BinaryOp {
    /// Binding strength. Higher binds tighter.
    pub(crate) fn precedence(self) -> u8 {
        match self {
            Self::Eq | Self::Ne | Self::Lt | Self::Le | Self::Gt | Self::Ge => 1,
            Self::Concat => 2,
            Self::Add | Self::Sub => 3,
            Self::Mul | Self::Div => 4,
            Self::Pow => 6,
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Pow => "^",
            Self::Concat => "&",
            Self::Eq => "=",
            Self::Ne => "<>",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
        }
    }
}

/// Binding strength of prefix operators, which bind tighter than `^` as in Excel.
pub(crate) const PREFIX_PRECEDENCE: u8 = 7;
/// Binding strength of `%`, which binds tighter than everything else.
pub(crate) const PERCENT_PRECEDENCE: u8 = 8;

/// A parsed formula. Displaying it gives equivalent formula text.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Text(String),
    Bool(bool),
    /// An error literal, such as `#REF!`.
    Error(String),
    Cell {
        sheet: Option<String>,
        cell: CellRef,
    },
    Range {
        sheet: Option<String>,
        start: CellRef,
        end: CellRef,
    },
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

impl Expr {
    fn precedence(&self) -> u8 {
        match self {
            Self::Binary(op, ..) => op.precedence(),
            Self::Unary(UnaryOp::Percent, _) => PERCENT_PRECEDENCE,
            Self::Unary(..) => PREFIX_PRECEDENCE,
            _ => u8::MAX,
        }
    }

    /// Visits this expression and everything inside it.
    pub fn walk<'a>(&'a self, visit: &mut impl FnMut(&'a Expr)) {
        visit(self);
        match self {
            Self::Unary(_, inner) => inner.walk(visit),
            Self::Binary(_, left, right) => {
                left.walk(visit);
                right.walk(visit);
            }
            Self::Call { args, .. } => args.iter().for_each(|arg| arg.walk(visit)),
            _ => {}
        }
    }
}

fn write_sheet(f: &mut fmt::Formatter<'_>, sheet: Option<&str>) -> fmt::Result {
    let Some(name) = sheet else {
        return Ok(());
    };
    let plain = name.chars().all(|c| c.is_alphanumeric() || c == '_')
        && name.chars().next().is_some_and(|c| !c.is_ascii_digit());
    if plain { write!(f, "{name}!") } else { write!(f, "'{}'!", name.replace('\'', "''")) }
}

fn write_operand(f: &mut fmt::Formatter<'_>, expr: &Expr, min: u8) -> fmt::Result {
    if expr.precedence() < min { write!(f, "({expr})") } else { write!(f, "{expr}") }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{n}"),
            Self::Text(text) => write!(f, "\"{}\"", text.replace('"', "\"\"")),
            Self::Bool(true) => f.write_str("TRUE"),
            Self::Bool(false) => f.write_str("FALSE"),
            Self::Error(text) => f.write_str(text),
            Self::Cell { sheet, cell } => {
                write_sheet(f, sheet.as_deref())?;
                write!(f, "{cell}")
            }
            Self::Range { sheet, start, end } => {
                write_sheet(f, sheet.as_deref())?;
                write!(f, "{start}:{end}")
            }
            Self::Unary(UnaryOp::Neg, inner) => {
                f.write_str("-")?;
                write_operand(f, inner, PREFIX_PRECEDENCE)
            }
            Self::Unary(UnaryOp::Plus, inner) => {
                f.write_str("+")?;
                write_operand(f, inner, PREFIX_PRECEDENCE)
            }
            Self::Unary(UnaryOp::Percent, inner) => {
                write_operand(f, inner, PERCENT_PRECEDENCE)?;
                f.write_str("%")
            }
            Self::Binary(op, left, right) => {
                // Operators are left associative, so a right operand of equal
                // strength keeps its parentheses.
                write_operand(f, left, op.precedence())?;
                f.write_str(op.symbol())?;
                write_operand(f, right, op.precedence() + 1)
            }
            Self::Call { name, args } => {
                write!(f, "{name}(")?;
                for (index, arg) in args.iter().enumerate() {
                    if index > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{arg}")?;
                }
                f.write_str(")")
            }
        }
    }
}
