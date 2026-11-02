use excel_model::{CellError, CellValue, Doc};

/// A function argument.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    /// A typed value, such as `5`, `"x"`, or the result of an expression.
    Value(CellValue),
    /// Values read from a cell or a range.
    Cells(Cells),
}

impl Arg {
    /// Cells holding `values` in one column, for tests and simple callers.
    pub fn cells(values: Vec<CellValue>) -> Self {
        let rows = values.len() as u32;
        Self::Cells(Cells::new(rows.max(1), 1, values.into_iter().enumerate().map(|(row, v)| (row as u32, 0, v))))
    }
}

/// The cells of a reference: its shape, and the values that are not empty
/// with their positions inside it. Empty cells are left out, so a whole column
/// costs only what it holds.
#[derive(Debug, Clone, PartialEq)]
pub struct Cells {
    pub rows: u32,
    pub cols: u32,
    /// Row and column offsets with values, in reading order.
    entries: Vec<(u32, u32, CellValue)>,
}

impl Cells {
    /// Builds cells from positioned values. Empty values are dropped.
    pub fn new(rows: u32, cols: u32, entries: impl IntoIterator<Item = (u32, u32, CellValue)>) -> Self {
        let mut entries: Vec<_> = entries.into_iter().filter(|(_, _, value)| !value.is_empty()).collect();
        entries.sort_by_key(|(row, col, _)| (*row, *col));
        Self { rows, cols, entries }
    }

    /// The values that are not empty, in reading order.
    pub fn values(&self) -> impl Iterator<Item = &CellValue> {
        self.entries.iter().map(|(_, _, value)| value)
    }

    /// The values that are not empty, with their positions.
    pub fn entries(&self) -> &[(u32, u32, CellValue)] {
        &self.entries
    }

    /// The value at a position, empty if nothing is there.
    pub fn get(&self, row: u32, col: u32) -> &CellValue {
        const EMPTY: CellValue = CellValue::Empty;
        match self.entries.binary_search_by_key(&(row, col), |(r, c, _)| (*r, *c)) {
            Ok(index) => &self.entries[index].2,
            Err(_) => &EMPTY,
        }
    }

    /// How many cells the reference covers, empty ones included.
    pub fn size(&self) -> u64 {
        u64::from(self.rows) * u64::from(self.cols)
    }
}

/// One value from an argument that stands for a single value. An empty cell
/// is 0, and a range of several cells is an error.
pub fn scalar(arg: &Arg) -> Result<CellValue, CellError> {
    match arg {
        Arg::Value(value) => Ok(value.clone()),
        Arg::Cells(cells) if cells.size() == 1 => Ok(match cells.get(0, 0) {
            CellValue::Empty => CellValue::Number(0.0),
            value => value.clone(),
        }),
        Arg::Cells(_) => Err(CellError::Value),
    }
}

/// One number from an argument that stands for a single value.
pub fn scalar_number(arg: &Arg) -> Result<f64, CellError> {
    to_number(&scalar(arg)?)
}

/// A spreadsheet function. Arguments are already evaluated, and errors arrive
/// as values, so `IF` can ignore an error in the branch it does not take.
pub trait Function {
    /// The uppercase name used in formulas.
    fn name(&self) -> &'static str;
    /// Help for the function. Its usage, such as `SUM(values...)`, and summary
    /// show during completion.
    fn doc(&self) -> Doc;
    fn call(&self, args: &[Arg]) -> CellValue;
    /// Whether the result can change with the same arguments, as `TODAY` does,
    /// so formulas calling it are recomputed after every edit.
    fn volatile(&self) -> bool {
        false
    }
    /// Whether the result is a date, so the formula shows it as one.
    fn returns_date(&self) -> bool {
        false
    }
}

/// Converts a value to a number the way arithmetic does.
pub fn to_number(value: &CellValue) -> Result<f64, CellError> {
    match value {
        CellValue::Number(n) => Ok(*n),
        CellValue::Bool(b) => Ok(f64::from(u8::from(*b))),
        CellValue::Empty => Ok(0.0),
        CellValue::Text(text) => text.trim().parse().ok().filter(|n: &f64| n.is_finite()).ok_or(CellError::Value),
        CellValue::Error(error) => Err(*error),
    }
}

/// Converts a value to a boolean the way conditions do.
pub fn to_bool(value: &CellValue) -> Result<bool, CellError> {
    match value {
        CellValue::Bool(b) => Ok(*b),
        CellValue::Number(n) => Ok(*n != 0.0),
        CellValue::Empty => Ok(false),
        CellValue::Text(text) if text.eq_ignore_ascii_case("TRUE") => Ok(true),
        CellValue::Text(text) if text.eq_ignore_ascii_case("FALSE") => Ok(false),
        CellValue::Text(_) => Err(CellError::Value),
        CellValue::Error(error) => Err(*error),
    }
}
