use excel_formula_functions::{Arg, Cells, FunctionRegistry, to_number};
use excel_formula_syntax::{BinaryOp, CellRef, Expr, UnaryOp};
use excel_model::{CellError, CellRange, CellValue, NumberFormat, Workbook};

/// Evaluates formulas, reading already-computed values from the workbook.
pub(crate) struct Evaluator<'a> {
    pub workbook: &'a Workbook,
    pub functions: &'a FunctionRegistry,
    /// The sheet holding the formula, used for references without a sheet name.
    pub sheet: usize,
}

impl Evaluator<'_> {
    pub fn eval(&self, expr: &Expr) -> CellValue {
        match expr {
            Expr::Number(n) => CellValue::Number(*n),
            Expr::Text(text) => CellValue::Text(text.clone()),
            Expr::Bool(b) => CellValue::Bool(*b),
            Expr::Error(text) => CellValue::Error(CellError::from_excel(text).unwrap_or(CellError::Name)),
            Expr::Cell { sheet, cell } => self.cell_value(sheet.as_deref(), *cell),
            // A range where one value is expected, as Excel reports without array support.
            Expr::Range { .. } => CellValue::Error(CellError::Value),
            Expr::Unary(op, inner) => self.unary(*op, inner),
            Expr::Binary(op, left, right) => self.binary(*op, left, right),
            Expr::Call { name, args } => match self.functions.get(name) {
                Some(function) => {
                    let args: Vec<Arg> = args.iter().map(|arg| self.argument(arg)).collect();
                    function.call(&args)
                }
                None => CellValue::Error(CellError::Name),
            },
        }
    }

    fn sheet_index(&self, name: Option<&str>) -> Option<usize> {
        match name {
            Some(name) => self.workbook.sheet_index(name),
            None => Some(self.sheet),
        }
    }

    /// Whether a formula gives a date, as Excel decides when picking a
    /// formula's format: a function returning a date, a reference to a date,
    /// or a date plus or minus days. The difference of two dates is a count
    /// of days, not a date.
    pub fn is_date(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Cell { sheet, cell } => self.sheet_index(sheet.as_deref()).is_some_and(|index| {
                self.workbook.sheets()[index].get(cell.addr).is_some_and(|cell| cell.format == NumberFormat::Date)
            }),
            Expr::Call { name, .. } => self.functions.get(name).is_some_and(|function| function.returns_date()),
            Expr::Unary(UnaryOp::Plus, inner) => self.is_date(inner),
            Expr::Binary(BinaryOp::Add, left, right) => self.is_date(left) != self.is_date(right),
            Expr::Binary(BinaryOp::Sub, left, right) => self.is_date(left) && !self.is_date(right),
            _ => false,
        }
    }

    fn cell_value(&self, sheet: Option<&str>, cell: CellRef) -> CellValue {
        match self.sheet_index(sheet) {
            Some(index) => self.workbook.sheets()[index].value(cell.addr).clone(),
            None => CellValue::Error(CellError::Ref),
        }
    }

    /// Evaluates a function argument. References keep their cell values
    /// separate from typed values, because functions treat them differently.
    fn argument(&self, expr: &Expr) -> Arg {
        match expr {
            Expr::Cell { sheet, cell } => match self.cell_value(sheet.as_deref(), *cell) {
                CellValue::Error(CellError::Ref) if self.sheet_index(sheet.as_deref()).is_none() => {
                    Arg::Value(CellValue::Error(CellError::Ref))
                }
                value => Arg::Cells(Cells::new(1, 1, [(0, 0, value)])),
            },
            Expr::Range { sheet, start, end } => match self.sheet_index(sheet.as_deref()) {
                Some(index) => {
                    let range = CellRange::new(start.addr, end.addr);
                    let entries = self.workbook.sheets()[index].iter_range(range).map(|(addr, cell)| {
                        (addr.row - range.start.row, addr.col - range.start.col, cell.value.clone())
                    });
                    Arg::Cells(Cells::new(range.rows(), range.cols(), entries))
                }
                None => Arg::Value(CellValue::Error(CellError::Ref)),
            },
            other => Arg::Value(self.eval(other)),
        }
    }

    fn unary(&self, op: UnaryOp, inner: &Expr) -> CellValue {
        let value = self.eval(inner);
        match to_number(&value) {
            Ok(n) => CellValue::Number(match op {
                UnaryOp::Neg => -n,
                UnaryOp::Plus => n,
                UnaryOp::Percent => n / 100.0,
            }),
            Err(error) => CellValue::Error(error),
        }
    }

    fn binary(&self, op: BinaryOp, left: &Expr, right: &Expr) -> CellValue {
        let left = self.eval(left);
        let right = self.eval(right);
        if let CellValue::Error(error) = left {
            return CellValue::Error(error);
        }
        if let CellValue::Error(error) = right {
            return CellValue::Error(error);
        }
        match op {
            BinaryOp::Concat => CellValue::Text(format!("{left}{right}")),
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                let ordering = compare(&left, &right);
                CellValue::Bool(match op {
                    BinaryOp::Eq => ordering.is_eq(),
                    BinaryOp::Ne => ordering.is_ne(),
                    BinaryOp::Lt => ordering.is_lt(),
                    BinaryOp::Le => ordering.is_le(),
                    BinaryOp::Gt => ordering.is_gt(),
                    _ => ordering.is_ge(),
                })
            }
            _ => arithmetic(op, &left, &right),
        }
    }
}

fn arithmetic(op: BinaryOp, left: &CellValue, right: &CellValue) -> CellValue {
    let (a, b) = match (to_number(left), to_number(right)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(error), _) | (_, Err(error)) => return CellValue::Error(error),
    };
    let result = match op {
        BinaryOp::Add => a + b,
        BinaryOp::Sub => a - b,
        BinaryOp::Mul => a * b,
        BinaryOp::Div if b == 0.0 => return CellValue::Error(CellError::Div0),
        BinaryOp::Div => a / b,
        BinaryOp::Pow if a == 0.0 && b == 0.0 => return CellValue::Error(CellError::Num),
        BinaryOp::Pow if a == 0.0 && b < 0.0 => return CellValue::Error(CellError::Div0),
        BinaryOp::Pow => a.powf(b),
        _ => unreachable!("only arithmetic operators reach here"),
    };
    if result.is_finite() { CellValue::Number(result) } else { CellValue::Error(CellError::Num) }
}

/// Orders values as Excel does: numbers, then text, then booleans. An empty
/// cell takes the type of what it is compared with. Text ignores case.
fn compare(left: &CellValue, right: &CellValue) -> std::cmp::Ordering {
    use CellValue::{Bool, Empty, Number, Text};
    let empty_like = |other: &CellValue| match other {
        Text(_) => Text(String::new()),
        Bool(_) => Bool(false),
        _ => Number(0.0),
    };
    let left = if left.is_empty() { empty_like(right) } else { left.clone() };
    let right = if right.is_empty() { empty_like(&left) } else { right.clone() };
    let rank = |value: &CellValue| match value {
        Number(_) | Empty => 0,
        Text(_) => 1,
        _ => 2,
    };
    match (&left, &right) {
        (Number(a), Number(b)) => a.total_cmp(b),
        (Text(a), Text(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
        (Bool(a), Bool(b)) => a.cmp(b),
        _ => rank(&left).cmp(&rank(&right)),
    }
}
