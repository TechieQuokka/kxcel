//! Tables, and the other things a sheet holds besides its cells.

use crate::{CellAddr, CellRange, CellValue, Criterion, MAX_COLS, MAX_ROWS, RangeSet, RowSet};

/// A named range whose first row holds column headers.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub name: String,
    /// The whole table, header row included.
    pub range: CellRange,
    /// The column of the last `table sort` and whether it was descending, kept
    /// only to mark that header. The table does not stay sorted.
    pub sorted: Option<(u32, bool)>,
    /// At most one filter for each column. Rows must meet all of them to show.
    pub filters: Vec<ColumnFilter>,
}

/// Which rows of a table show, judged by one column.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnFilter {
    /// The sheet column, not its place in the table.
    pub column: u32,
    pub rule: FilterRule,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterRule {
    /// Rows whose cell shows one of these, ignoring case. An empty string
    /// stands for blank cells.
    Values(Vec<String>),
    /// Rows whose cell meets a condition written as for `COUNTIF`, such as
    /// `>5000`, `<>서울`, or `사과*`.
    Condition(String),
}

impl FilterRule {
    /// Whether a cell, given by its value and the text it shows, passes.
    pub fn passes(&self, value: &CellValue, shown: &str) -> bool {
        match self {
            Self::Values(values) => values.iter().any(|wanted| wanted.trim().eq_ignore_ascii_case(shown.trim())),
            Self::Condition(condition) => Criterion::new(&CellValue::Text(condition.clone())).matches(value),
        }
    }
}

impl Table {
    pub fn new(name: impl Into<String>, range: CellRange) -> Self {
        Self { name: name.into(), range, sorted: None, filters: Vec::new() }
    }

    pub fn header_row(&self) -> u32 {
        self.range.start.row
    }

    /// True when `addr` is in the header row.
    pub fn is_header(&self, addr: CellAddr) -> bool {
        self.range.contains(addr) && addr.row == self.header_row()
    }

    /// The data rows, below the header. Empty when the table is only a header.
    pub fn data_rows(&self) -> std::ops::RangeInclusive<u32> {
        self.range.start.row + 1..=self.range.end.row
    }
}

/// Everything a sheet holds besides cells and column widths. Changing it alone
/// never changes a formula's result, so it can be undone without recalculating.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SheetMeta {
    pub tables: Vec<Table>,
    /// Rows hidden by table filters. Kept as set when filters were last
    /// applied, so editing a cell never makes its row vanish.
    pub hidden_rows: RowSet,
    /// Cells an app lets its users change, empty ones included.
    pub input_cells: RangeSet,
}

/// Rows or columns inserted or deleted.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Shift {
    pub rows: bool,
    pub insert: bool,
    pub at: u32,
    pub count: u32,
}

impl Shift {
    /// Where one row or column is now, or `None` when it was deleted or pushed
    /// off the sheet.
    fn point(self, n: u32) -> Option<u32> {
        let limit = if self.rows { MAX_ROWS } else { MAX_COLS };
        if n < self.at {
            Some(n)
        } else if self.insert {
            n.checked_add(self.count).filter(|n| *n < limit)
        } else if n < self.at.saturating_add(self.count) {
            None
        } else {
            Some(n - self.count)
        }
    }

    /// Where a range is now, or `None` when it is gone. With `header_first`,
    /// a range also goes when its first row does, as a table does.
    fn range(self, range: CellRange, header_first: bool) -> Option<CellRange> {
        if self.rows {
            let (first, last) = self.span(range.start.row, range.end.row, header_first)?;
            Some(CellRange { start: CellAddr::new(first, range.start.col), end: CellAddr::new(last, range.end.col) })
        } else {
            let (first, last) = self.span(range.start.col, range.end.col, false)?;
            Some(CellRange { start: CellAddr::new(range.start.row, first), end: CellAddr::new(range.end.row, last) })
        }
    }

    /// Where a span of rows or columns is now, or `None` when it is gone.
    fn span(self, first: u32, last: u32, header_first: bool) -> Option<(u32, u32)> {
        let limit = if self.rows { MAX_ROWS } else { MAX_COLS };
        if self.insert {
            insert_span((first, last), self.at, self.count, limit)
        } else {
            delete_span((first, last), self.at, self.count, header_first)
        }
    }
}

impl SheetMeta {
    /// Moves everything after rows or columns were inserted or deleted. A table
    /// goes when its header row or all its columns do.
    pub(crate) fn shift(&mut self, shift: Shift) {
        self.tables.retain_mut(|table| {
            let Some(moved) = shift.range(table.range, true) else {
                return false;
            };
            table.range = moved;
            if !shift.rows {
                table.sorted = table.sorted.and_then(|(col, descending)| Some((shift.point(col)?, descending)));
                table.filters.retain_mut(|filter| match shift.point(filter.column) {
                    Some(col) => {
                        filter.column = col;
                        true
                    }
                    None => false,
                });
            }
            true
        });
        self.input_cells.remap(|range| shift.range(range, false));
        match (shift.rows, shift.insert) {
            (true, true) => self.hidden_rows.insert_rows(shift.at, shift.count),
            (true, false) => self.hidden_rows.delete_rows(shift.at, shift.count),
            _ => {}
        }
    }
}

/// Where a span of rows or columns, `(first, last)`, is after `count` were
/// inserted at `at`. Inserting inside the span widens it. `None` when it is
/// pushed off the sheet.
fn insert_span((first, last): (u32, u32), at: u32, count: u32, limit: u32) -> Option<(u32, u32)> {
    let shift = |n: u32| n.checked_add(count).filter(|n| *n < limit);
    if at <= first {
        let first = shift(first)?;
        Some((first, shift(last).unwrap_or(limit - 1)))
    } else if at <= last {
        Some((first, shift(last).unwrap_or(limit - 1)))
    } else {
        Some((first, last))
    }
}

/// Where a span is after `count` rows or columns at `at` were deleted. `None`
/// when all of it is gone, or with `keep_first`, when its first one is, as a
/// table goes when its header row does.
fn delete_span((first, last): (u32, u32), at: u32, count: u32, keep_first: bool) -> Option<(u32, u32)> {
    let end = at.saturating_add(count);
    if end <= first {
        return Some((first - count, last - count));
    }
    if at > last {
        return Some((first, last));
    }
    if keep_first && at <= first {
        return None;
    }
    let removed_up_to_last = end.min(last + 1) - at;
    let removed_before_first = if at < first { end.min(first) - at } else { 0 };
    let (first, last) = (first - removed_before_first, last.checked_sub(removed_up_to_last)?);
    (last >= first).then_some((first, last))
}

/// Why a table name cannot be used, or `None` when it can. Follows Excel's
/// rules: a letter or underscore first, then letters, digits, underscores, or
/// periods, and nothing that reads as a cell address.
pub fn table_name_problem(name: &str) -> Option<String> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Some("A table name cannot be empty".into());
    };
    if !(first.is_alphabetic() || first == '_' || first == '\\') {
        return Some(format!("A table name must start with a letter or underscore: {name}"));
    }
    if let Some(bad) = chars.find(|c| !(c.is_alphanumeric() || *c == '_' || *c == '.')) {
        return Some(format!("A table name cannot contain '{bad}': {name}"));
    }
    if name.chars().count() > 255 {
        return Some("A table name can have at most 255 characters".into());
    }
    let reads_as_reference = name.parse::<CellAddr>().is_ok()
        || ["r", "c"].iter().any(|letter| name.eq_ignore_ascii_case(letter))
        || is_r1c1(name);
    reads_as_reference.then(|| format!("A table name cannot look like a cell reference: {name}"))
}

/// True for names such as `R1C1`, `R2`, or `C3`, which Excel reads as references.
fn is_r1c1(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    let all_digits = |text: &str| text.chars().all(|c| c.is_ascii_digit());
    if let Some(rest) = upper.strip_prefix('R') {
        let split = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        let after = &rest[split..];
        return if after.is_empty() { !rest.is_empty() } else { after.strip_prefix('C').is_some_and(all_digits) };
    }
    upper.strip_prefix('C').is_some_and(|rest| !rest.is_empty() && all_digits(rest))
}

/// Header names for a new table, as Excel gives them: an empty header becomes
/// `Column<n>`, numbered by its place in the table, and a header that repeats
/// an earlier one, ignoring case, gets a number added.
pub fn header_names(headers: &[String]) -> Vec<String> {
    let mut names: Vec<String> = Vec::with_capacity(headers.len());
    for (index, header) in headers.iter().enumerate() {
        let header = header.trim();
        let base = if header.is_empty() { format!("Column{}", index + 1) } else { header.to_owned() };
        // A made-up name also stays clear of every real header.
        let clashes = |name: &str| {
            names.iter().any(|taken| taken.eq_ignore_ascii_case(name))
                || (header.is_empty() && headers.iter().any(|real| real.trim().eq_ignore_ascii_case(name)))
        };
        let mut name = base.clone();
        let mut n = 2;
        while clashes(&name) {
            name = format!("{base}{n}");
            n += 1;
        }
        names.push(name);
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn valid_and_invalid_names() {
        for good in ["Table1", "판매", "_x", "Sales.2026", "A1B", "RC1x"] {
            assert_eq!(table_name_problem(good), None, "{good}");
        }
        for bad in ["", "1Table", "has space", "A1", "XFD100", "R", "c", "R1C1", "R2", "C3", "a-b"] {
            assert!(table_name_problem(bad).is_some(), "{bad}");
        }
    }

    #[test]
    fn headers_fill_gaps_and_repeats() {
        assert_eq!(header_names(&strings(&["이름", "", "가격"])), ["이름", "Column2", "가격"]);
        assert_eq!(header_names(&strings(&["a", "A", "b"])), ["a", "A2", "b"]);
        assert_eq!(header_names(&strings(&["", "Column1"])), ["Column12", "Column1"]);
    }

    #[test]
    fn shifting_moves_sort_marks_filters_and_hidden_rows() {
        let mut table = Table::new("T", "B1:D9".parse().unwrap());
        table.sorted = Some((3, true));
        table.filters = vec![
            ColumnFilter { column: 2, rule: FilterRule::Condition(">1".into()) },
            ColumnFilter { column: 3, rule: FilterRule::Values(vec!["a".into()]) },
        ];
        let mut meta =
            SheetMeta { tables: vec![table], hidden_rows: [2, 5, 7].into_iter().collect(), ..Default::default() };
        meta.shift(Shift { rows: false, insert: true, at: 0, count: 1 });
        assert_eq!(meta.tables[0].sorted, Some((4, true)));
        meta.shift(Shift { rows: false, insert: false, at: 3, count: 1 });
        assert_eq!(meta.tables[0].range.to_string(), "C1:D9");
        assert_eq!(meta.tables[0].filters.iter().map(|f| f.column).collect::<Vec<_>>(), [3]);
        assert_eq!(meta.tables[0].sorted, Some((3, true)));
        meta.shift(Shift { rows: true, insert: false, at: 5, count: 1 });
        assert_eq!(meta.hidden_rows.iter().collect::<Vec<_>>(), [2, 6]);
        meta.shift(Shift { rows: true, insert: true, at: 0, count: 2 });
        assert_eq!(meta.hidden_rows.iter().collect::<Vec<_>>(), [4, 8]);
    }

    #[test]
    fn filter_rules() {
        let values = FilterRule::Values(vec!["서울".into(), "".into()]);
        assert!(values.passes(&CellValue::Text("서울".into()), "서울"));
        assert!(values.passes(&CellValue::Empty, ""));
        assert!(!values.passes(&CellValue::Text("부산".into()), "부산"));
        let condition = FilterRule::Condition(">5000".into());
        assert!(condition.passes(&CellValue::Number(6000.0), "6000"));
        assert!(!condition.passes(&CellValue::Number(10.0), "10"));
        assert!(FilterRule::Condition("사과*".into()).passes(&CellValue::Text("사과즙".into()), "사과즙"));
    }
}
