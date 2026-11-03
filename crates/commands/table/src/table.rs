use excel_command::{Command, CommandError, Context};
use excel_edit::{CellChange, Edit};
use excel_model::{
    Cell, CellAddr, CellRange, CellValue, ColumnFilter, Doc, FilterRule, Sheet, Table, header_names, table_name_problem,
};

pub(crate) struct TableCommand;

const USAGE: &str = "Usage: table [create [name] | remove | resize <range> | sort <column> [desc] | filter ...]";

impl Command for TableCommand {
    fn name(&self) -> &str {
        "table"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Table",
            summary: "Create, remove, resize, sort, filter, or list tables",
            usage: "table [create [name] | remove | resize <range> | sort <column> [desc] | filter <column> <values...>]",
            description: "A table is a range whose first row holds column headers. create makes the \
                          selected range a table, leaving out empty rows and columns at its edges. Empty \
                          or repeated headers get names such as Column3. Names default to Table1, Table2, \
                          and so on. A value typed just below a table adds a row, and a header typed just \
                          right of it adds a column. remove, resize, and sort act on the table at the \
                          cursor; remove leaves its cells as they are. sort orders the rows below the \
                          header by the column with that header, once: the table does not stay sorted. With nothing after it, lists the tables on \
                          the sheet. Ctrl+T, or Create table on the right-click menu, also creates a table, \
                          and dragging the mark at a table's bottom right corner resizes it.",
            args: &[
                ("create [name]", "Optional. Makes a table, named as given or TableN"),
                ("remove", "Optional. The table at the cursor stops being a table"),
                ("resize <range>", "Optional. Sets the table's range. Its header row stays first"),
                ("sort <column> [desc]", "Optional. Sorts by the column with this header, smallest first or desc"),
                ("filter <column> <values...>", "Optional. Shows rows with one of these values in the column"),
                ("filter <column> <condition>", "Optional. Shows rows meeting a condition such as >5000"),
                ("filter [<column>] clear", "Optional. Removes the column's filter, or every filter"),
            ],
            examples: &[
                ("table create", ""),
                ("table create 매출", ""),
                ("table resize A1:D20", ""),
                ("table sort 가격 desc", ""),
                ("table filter 지역 서울 부산", ""),
                ("table filter 가격 >5000", ""),
                ("table filter clear", ""),
                ("table", ""),
            ],
            related: &["sort"],
        }
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        match args {
            [] => list(ctx),
            ["create"] => create(ctx, None),
            ["create", name] => create(ctx, Some(name)),
            ["remove"] => remove(ctx),
            ["resize", range] => resize(ctx, range),
            ["sort", words @ ..] if !words.is_empty() => sort(ctx, words),
            ["filter", "clear"] => filter(ctx, None, None),
            ["filter", column, "clear"] => filter(ctx, Some(column), None),
            ["filter", column, values @ ..] if !values.is_empty() => filter(ctx, Some(column), Some(rule(values))),
            _ => Err(CommandError::new(USAGE)),
        }
    }
}

fn list(ctx: &mut Context<'_>) -> Result<(), CommandError> {
    let tables = ctx.doc.workbook.active().tables();
    if tables.is_empty() {
        ctx.say("No tables on this sheet. table create makes one");
    } else {
        let names: Vec<String> = tables
            .iter()
            .map(|table| match table.filters.len() {
                0 => format!("{} {}", table.name, table.range),
                1 => format!("{} {} (1 filter)", table.name, table.range),
                n => format!("{} {} ({n} filters)", table.name, table.range),
            })
            .collect();
        ctx.say(names.join("  "));
    }
    Ok(())
}

fn create(ctx: &mut Context<'_>, name: Option<&str>) -> Result<(), CommandError> {
    let sheet = ctx.doc.workbook.active();
    if ctx.selection.is_single() {
        return Err(CommandError::new("Select the range for the table first"));
    }
    // Empty rows and columns at the selection's edges are left out.
    let range = sheet
        .filled_bounds(ctx.selection)
        .ok_or_else(|| CommandError::new(format!("The selected range {} is empty", ctx.selection)))?;
    if let Some(other) = sheet.tables().iter().find(|table| table.range.overlaps(range)) {
        return Err(CommandError::new(format!("{range} overlaps table {}", other.name)));
    }
    let name = match name {
        Some(name) => {
            check_name(ctx, name, None)?;
            name.to_owned()
        }
        None => next_name(ctx),
    };

    let index = ctx.doc.workbook.active_index();
    let before = sheet.meta().clone();
    let mut after = before.clone();
    after.tables.push(Table::new(name.clone(), range));
    let headers = header_changes(sheet, index, range);
    ctx.apply(batch(headers, Edit::SetMeta { sheet: index, before, after }));
    ctx.say(format!("Created table {name} at {range}"));
    Ok(())
}

fn remove(ctx: &mut Context<'_>) -> Result<(), CommandError> {
    let (position, table) = table_at_cursor(ctx)?;
    let index = ctx.doc.workbook.active_index();
    let before = ctx.doc.workbook.active().meta().clone();
    let mut after = before.clone();
    after.tables.remove(position);
    after.hidden_rows = ctx.doc.workbook.active().rows_hidden_by(&after.tables);
    ctx.apply(Edit::SetMeta { sheet: index, before, after });
    ctx.say(format!("Removed table {}. Its cells stay", table.name));
    Ok(())
}

fn resize(ctx: &mut Context<'_>, text: &str) -> Result<(), CommandError> {
    let range: CellRange = text.parse().map_err(CommandError::new)?;
    // The table at the cursor, or else the one starting where the range does,
    // as when its corner is dragged.
    let (position, table) = table_at_cursor(ctx).or_else(|error| {
        let tables = ctx.doc.workbook.active().tables();
        tables
            .iter()
            .position(|table| table.range.start == range.start)
            .map(|position| (position, tables[position].clone()))
            .ok_or(error)
    })?;
    if range.start.row != table.header_row() {
        return Err(CommandError::new(format!(
            "The header row must stay first: the range must start in row {}",
            table.header_row() + 1
        )));
    }
    let sheet = ctx.doc.workbook.active();
    if let Some(other) =
        sheet.tables().iter().enumerate().find(|(i, other)| *i != position && other.range.overlaps(range))
    {
        return Err(CommandError::new(format!("{range} overlaps table {}", other.1.name)));
    }
    let index = ctx.doc.workbook.active_index();
    let before = sheet.meta().clone();
    let mut after = before.clone();
    after.tables[position].range = range;
    after.hidden_rows = sheet.rows_hidden_by(&after.tables);
    let headers = header_changes(sheet, index, range);
    ctx.apply(batch(headers, Edit::SetMeta { sheet: index, before, after }));
    ctx.say(format!("Table {} is now {range}", table.name));
    Ok(())
}

/// Sorts the table's data rows by a column named by its header. A trailing
/// `desc` or `asc` gives the direction, and the words before it the header,
/// so a header with spaces needs no quotes.
fn sort(ctx: &mut Context<'_>, words: &[&str]) -> Result<(), CommandError> {
    let (descending, words) = match words.split_last() {
        Some((last, rest)) if !rest.is_empty() && last.eq_ignore_ascii_case("desc") => (true, rest),
        Some((last, rest)) if !rest.is_empty() && last.eq_ignore_ascii_case("asc") => (false, rest),
        _ => (false, words),
    };
    let header = words.join(" ");
    let (position, table) = table_at_cursor(ctx)?;
    let col = column_of(ctx.doc.workbook.active(), &table, &header)?;
    if table.range.rows() < 2 {
        return Err(CommandError::new(format!("Table {} has no rows below its header", table.name)));
    }
    let rows = CellRange { start: CellAddr::new(table.header_row() + 1, table.range.start.col), end: table.range.end };
    let active = ctx.doc.workbook.active_index();
    let calculator = ctx.calculator;
    let edit = Edit::replacing_sheets(&ctx.doc.workbook, |sheets| {
        let sheet = &mut sheets[active];
        sheet.sort_rows(rows, col, descending, |text, moved| calculator.translate(text, moved, 0));
        let mut meta = sheet.meta().clone();
        meta.tables[position].sorted = Some((col, descending));
        // Filters apply again, so what shows is still what matches.
        meta.hidden_rows = sheet.rows_hidden_by(&meta.tables);
        sheet.set_meta(meta);
    });
    ctx.apply(edit);
    let direction = if descending { ", largest first" } else { "" };
    ctx.say(format!("Sorted table {} by {header}{direction}", table.name));
    Ok(())
}

/// Reads filter arguments. One argument that starts with `<`, `>`, or `=`,
/// holds a wildcard, or is empty is a condition. Anything else is a list of
/// values to show.
fn rule(values: &[&str]) -> FilterRule {
    match values {
        [one] if one.is_empty() || one.starts_with(['<', '>', '=']) || one.contains(['*', '?', '~']) => {
            FilterRule::Condition((*one).to_owned())
        }
        _ => FilterRule::Values(values.iter().map(|value| (*value).to_owned()).collect()),
    }
}

/// Sets or clears a filter on the table at the cursor and applies every
/// filter again. `column` `None` clears all of them, and `rule` `None` clears
/// that column's.
fn filter(ctx: &mut Context<'_>, column: Option<&str>, rule: Option<FilterRule>) -> Result<(), CommandError> {
    let (position, table) = table_at_cursor(ctx)?;
    let sheet = ctx.doc.workbook.active();
    let index = ctx.doc.workbook.active_index();
    let before = sheet.meta().clone();
    let mut after = before.clone();
    let filters = &mut after.tables[position].filters;
    let what = match (column, rule) {
        (None, _) => {
            filters.clear();
            "Cleared the filters".to_owned()
        }
        (Some(name), rule) => {
            let col = column_of(sheet, &table, name)?;
            filters.retain(|filter| filter.column != col);
            match rule {
                Some(rule) => {
                    filters.push(ColumnFilter { column: col, rule });
                    format!("Filtered by {name}")
                }
                None => format!("Cleared the filter on {name}"),
            }
        }
    };
    after.hidden_rows = sheet.rows_hidden_by(&after.tables);
    let total = table.range.rows() - 1;
    let hidden = after.hidden_rows.count_within(table.header_row() + 1, table.range.end.row);
    ctx.apply(Edit::SetMeta { sheet: index, before, after });
    ctx.say(format!("{what}: {} of {total} rows show in {}", u64::from(total) - hidden, table.name));
    Ok(())
}

/// The column whose header is `name`, or an error listing the headers.
fn column_of(sheet: &Sheet, table: &Table, name: &str) -> Result<u32, CommandError> {
    sheet.table_column(table, name).ok_or_else(|| {
        let headers: Vec<String> = (table.range.start.col..=table.range.end.col)
            .map(|col| sheet.display(CellAddr::new(table.header_row(), col)))
            .collect();
        CommandError::new(format!("Table {} has no column {name}. Its columns: {}", table.name, headers.join(", ")))
    })
}

/// The table at the cursor, with its position in the sheet's list.
fn table_at_cursor(ctx: &Context<'_>) -> Result<(usize, Table), CommandError> {
    let tables = ctx.doc.workbook.active().tables();
    tables
        .iter()
        .position(|table| table.range.contains(ctx.cursor))
        .map(|position| (position, tables[position].clone()))
        .ok_or_else(|| CommandError::new("The cursor is not in a table"))
}

/// Header cells to write so every column has a name of its own.
fn header_changes(sheet: &Sheet, index: usize, range: CellRange) -> Vec<CellChange> {
    let row = range.start.row;
    let addrs: Vec<CellAddr> = (range.start.col..=range.end.col).map(|col| CellAddr::new(row, col)).collect();
    let shown: Vec<String> = addrs.iter().map(|addr| sheet.display(*addr)).collect();
    addrs
        .iter()
        .zip(shown.iter().zip(header_names(&shown)))
        .filter(|(_, (shown, name))| shown.trim() != name)
        .map(|(addr, (_, name))| CellChange {
            sheet: index,
            addr: *addr,
            before: sheet.get(*addr).cloned(),
            after: Some(Cell::literal(CellValue::Text(name))),
        })
        .collect()
}

fn batch(cells: Vec<CellChange>, meta: Edit) -> Edit {
    if cells.is_empty() { meta } else { Edit::Batch(vec![Edit::SetCells(cells), meta]) }
}

/// Names must be unique in the workbook, ignoring case. `except` is a table
/// that may keep its own name.
fn check_name(ctx: &Context<'_>, name: &str, except: Option<&str>) -> Result<(), CommandError> {
    if let Some(problem) = table_name_problem(name) {
        return Err(CommandError::new(problem));
    }
    let taken =
        all_tables(ctx).any(|table| table.name.eq_ignore_ascii_case(name) && Some(table.name.as_str()) != except);
    if taken {
        return Err(CommandError::new(format!("A table named {name} already exists")));
    }
    Ok(())
}

fn next_name(ctx: &Context<'_>) -> String {
    (1..)
        .map(|n| format!("Table{n}"))
        .find(|name| !all_tables(ctx).any(|table| table.name.eq_ignore_ascii_case(name)))
        .expect("some number is free")
}

fn all_tables<'a>(ctx: &'a Context<'_>) -> impl Iterator<Item = &'a Table> {
    ctx.doc.workbook.sheets().iter().flat_map(|sheet: &Sheet| sheet.tables())
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{NoCalculator, NoStore};
    use excel_edit::Document;

    use super::*;

    fn addr(text: &str) -> CellAddr {
        text.parse().unwrap()
    }

    /// 이름 | (empty) | 가격 over two data rows, at A1:C3.
    fn doc() -> Document {
        let mut doc = Document::default();
        for (at, text) in
            [("A1", "이름"), ("C1", "가격"), ("A2", "사과"), ("B2", "1"), ("C2", "100"), ("A3", "배"), ("C3", "200")]
        {
            doc.workbook.active_mut().set(addr(at), Cell::parse_input(text));
        }
        doc
    }

    fn run_at(
        doc: &mut Document,
        cursor: &str,
        selection: Option<&str>,
        args: &[&str],
    ) -> Result<String, CommandError> {
        let mut ctx = Context::new(doc, &NoCalculator, &NoStore);
        ctx.cursor = addr(cursor);
        ctx.selection = selection.map_or(CellRange::single(ctx.cursor), |range| range.parse().unwrap());
        TableCommand.run(&mut ctx, args)?;
        Ok(ctx.message.unwrap_or_default())
    }

    fn ranges(doc: &Document) -> Vec<String> {
        doc.workbook.active().tables().iter().map(|t| format!("{} {}", t.name, t.range)).collect()
    }

    #[test]
    fn creates_from_the_selection_trimming_empty_edges_and_names_headers() {
        let mut d = doc();
        assert_eq!(run_at(&mut d, "A1", Some("A1:E9"), &["create"]).unwrap(), "Created table Table1 at A1:C3");
        assert_eq!(d.workbook.active().display(addr("B1")), "Column2");
        assert_eq!(run_at(&mut d, "A1", None, &[]).unwrap(), "Table1 A1:C3");
    }

    #[test]
    fn one_cell_or_an_empty_range_makes_no_table() {
        let mut d = doc();
        assert!(run_at(&mut d, "B2", None, &["create"]).unwrap_err().message.contains("Select the range"));
        assert!(run_at(&mut d, "H1", Some("H1:J5"), &["create"]).unwrap_err().message.contains("H1:J5 is empty"));
        assert!(ranges(&d).is_empty());
    }

    #[test]
    fn resize_finds_the_table_by_its_corner_when_the_cursor_is_elsewhere() {
        let mut d = doc();
        run_at(&mut d, "A1", Some("A1:C3"), &["create"]).unwrap();
        run_at(&mut d, "H9", None, &["resize", "A1:C5"]).unwrap();
        assert_eq!(ranges(&d), ["Table1 A1:C5"]);
        assert!(run_at(&mut d, "H9", None, &["resize", "B1:C5"]).unwrap_err().message.contains("not in a table"));
    }

    #[test]
    fn creates_from_a_selection_with_a_name() {
        let mut d = doc();
        run_at(&mut d, "A1", Some("A1:A3"), &["create", "과일"]).unwrap();
        assert_eq!(ranges(&d), ["과일 A1:A3"]);
        d.workbook.active_mut().set(addr("E2"), Cell::parse_input("1"));
        run_at(&mut d, "E1", Some("E1:F2"), &["create"]).unwrap();
        assert_eq!(ranges(&d), ["과일 A1:A3", "Table1 E2"], "the empty edges of E1:F2 are left out");
    }

    #[test]
    fn refuses_overlaps_bad_names_and_empty_places() {
        let mut d = doc();
        run_at(&mut d, "A1", Some("A1:C3"), &["create", "T"]).unwrap();
        d.workbook.active_mut().set(addr("D4"), Cell::parse_input("x"));
        assert!(run_at(&mut d, "C3", Some("C3:D4"), &["create"]).unwrap_err().message.contains("overlaps table T"));
        d.workbook.active_mut().set(addr("H1"), Cell::parse_input("x"));
        assert!(run_at(&mut d, "H1", Some("H1:H2"), &["create", "t"]).unwrap_err().message.contains("already exists"));
        assert!(run_at(&mut d, "H1", Some("H1:H2"), &["create", "A1"]).is_err());
        assert!(run_at(&mut d, "A1", None, &["nope"]).is_err());
    }

    #[test]
    fn names_are_unique_across_sheets() {
        let mut d = doc();
        run_at(&mut d, "A1", Some("A1:C3"), &["create"]).unwrap();
        let first = d.workbook.active().clone();
        let mut second = Sheet::new("Two");
        second.set(addr("A1"), Cell::parse_input("x"));
        d.workbook.replace_sheets(vec![first, second]);
        d.workbook.set_active(1);
        assert_eq!(run_at(&mut d, "A1", Some("A1:B2"), &["create"]).unwrap(), "Created table Table2 at A1");
    }

    #[test]
    fn removes_and_resizes_the_table_at_the_cursor() {
        let mut d = doc();
        run_at(&mut d, "A1", Some("A1:C3"), &["create"]).unwrap();
        assert_eq!(run_at(&mut d, "B2", None, &["resize", "A1:D5"]).unwrap(), "Table Table1 is now A1:D5");
        assert_eq!(d.workbook.active().display(addr("D1")), "Column4");
        assert!(run_at(&mut d, "B2", None, &["resize", "A2:D5"]).unwrap_err().message.contains("header row"));
        assert!(run_at(&mut d, "H9", None, &["remove"]).unwrap_err().message.contains("not in a table"));
        run_at(&mut d, "B2", None, &["remove"]).unwrap();
        assert!(ranges(&d).is_empty());
        assert_eq!(d.workbook.active().display(addr("A2")), "사과", "cells stay");
        assert_eq!(run_at(&mut d, "A1", None, &[]).unwrap(), "No tables on this sheet. table create makes one");
    }

    #[test]
    fn each_change_is_one_undo_step() {
        let mut d = doc();
        run_at(&mut d, "A1", Some("A1:C3"), &["create"]).unwrap();
        run_at(&mut d, "A1", None, &["resize", "A1:C4"]).unwrap();
        run_at(&mut d, "A1", None, &["remove"]).unwrap();
        d.undo();
        assert_eq!(ranges(&d), ["Table1 A1:C4"]);
        d.undo();
        assert_eq!(ranges(&d), ["Table1 A1:C3"]);
        d.undo();
        assert!(ranges(&d).is_empty());
        assert_eq!(d.workbook.active().display(addr("B1")), "", "the made-up header goes too");
    }
}

#[cfg(test)]
mod sort_tests {
    use excel_command::Calculator;
    use excel_command::testing::NoStore;
    use excel_edit::Document;

    use super::*;

    /// Moves references by the rows given, enough to check formulas move.
    struct Shift;

    impl Calculator for Shift {
        fn recalculate(&self, _: &mut excel_model::Workbook) {}
        fn translate(&self, formula: &str, rows: i64, _: i64) -> String {
            format!("{formula}{rows:+}")
        }
        fn adjust(&self, formula: &str, _: &str, _: &str, _: excel_command::Structural) -> String {
            formula.to_owned()
        }
        fn rename_sheet(&self, formula: &str, _: &str, _: &str) -> String {
            formula.to_owned()
        }
        fn remove_sheet(&self, formula: &str, _: &str) -> String {
            formula.to_owned()
        }
    }

    fn addr(text: &str) -> CellAddr {
        text.parse().unwrap()
    }

    /// A table at B2:C6 with headers 과일 and 단가 합계, and an empty price.
    fn doc() -> Document {
        let mut doc = Document::default();
        let cells = [
            ("B2", "과일"),
            ("C2", "단가 합계"),
            ("B3", "배"),
            ("C3", "300"),
            ("B4", "사과"),
            ("B5", "감"),
            ("C5", "100"),
            ("B6", "귤"),
            ("C6", "=C5"),
            ("A3", "outside"),
        ];
        for (at, text) in cells {
            doc.workbook.active_mut().set(addr(at), Cell::parse_input(text));
        }
        doc.workbook.active_mut().get_mut(addr("C6")).unwrap().value = CellValue::Number(200.0);
        let meta =
            excel_model::SheetMeta { tables: vec![Table::new("T", "B2:C6".parse().unwrap())], ..Default::default() };
        doc.workbook.active_mut().set_meta(meta);
        doc
    }

    fn run(doc: &mut Document, cursor: &str, args: &[&str]) -> Result<String, CommandError> {
        let mut ctx = Context::new(doc, &Shift, &NoStore);
        ctx.cursor = addr(cursor);
        TableCommand.run(&mut ctx, args)?;
        Ok(ctx.message.unwrap_or_default())
    }

    fn fruits(doc: &Document) -> Vec<String> {
        (3..=6).map(|row| doc.workbook.active().display(addr(&format!("B{row}")))).collect()
    }

    #[test]
    fn sorts_by_header_name_both_ways_with_empties_last() {
        let mut d = doc();
        assert_eq!(run(&mut d, "C4", &["sort", "단가", "합계"]).unwrap(), "Sorted table T by 단가 합계");
        assert_eq!(fruits(&d), ["감", "귤", "배", "사과"]);
        assert_eq!(d.workbook.active().display(addr("B2")), "과일", "the header stays");
        assert_eq!(d.workbook.active().display(addr("A3")), "outside", "cells outside stay");
        assert_eq!(d.workbook.active().tables()[0].sorted, Some((2, false)));
        run(&mut d, "C4", &["sort", "단가 합계", "DESC"]).unwrap();
        assert_eq!(fruits(&d), ["배", "귤", "감", "사과"]);
        assert_eq!(d.workbook.active().tables()[0].sorted, Some((2, true)));
        run(&mut d, "C4", &["sort", "과일"]).unwrap();
        assert_eq!(fruits(&d), ["감", "귤", "배", "사과"]);
    }

    #[test]
    fn formulas_move_with_their_rows() {
        let mut d = doc();
        run(&mut d, "B2", &["sort", "단가 합계"]).unwrap();
        let moved = d.workbook.active().get(addr("C4")).unwrap().input_text();
        assert_eq!(moved, "=C5-2", "the =C5 from row 6 moved up two rows");
    }

    #[test]
    fn errors_and_undo() {
        let mut d = doc();
        let error = run(&mut d, "B3", &["sort", "무게"]).unwrap_err().message;
        assert!(error.contains("no column 무게") && error.contains("과일, 단가 합계"), "{error}");
        assert!(run(&mut d, "A9", &["sort", "과일"]).unwrap_err().message.contains("not in a table"));
        assert!(run(&mut d, "B3", &["sort"]).is_err());
        run(&mut d, "B3", &["sort", "과일"]).unwrap();
        d.undo();
        assert_eq!(fruits(&d), ["배", "사과", "감", "귤"]);
        assert_eq!(d.workbook.active().tables()[0].sorted, None);
    }
}

#[cfg(test)]
mod filter_tests {
    use excel_command::testing::{NoCalculator, NoStore};
    use excel_edit::Document;

    use super::*;

    fn addr(text: &str) -> CellAddr {
        text.parse().unwrap()
    }

    /// 지역 | 가격 over five rows at A1:B6.
    fn doc() -> Document {
        let mut doc = Document::default();
        let rows = [("서울", "7000"), ("부산", "3000"), ("서울", "2000"), ("대구", "9000"), ("", "6000")];
        let sheet = doc.workbook.active_mut();
        sheet.set(addr("A1"), Cell::parse_input("지역"));
        sheet.set(addr("B1"), Cell::parse_input("가격"));
        for (i, (place, price)) in rows.iter().enumerate() {
            sheet.set(CellAddr::new(i as u32 + 1, 0), Cell::parse_input(place));
            sheet.set(CellAddr::new(i as u32 + 1, 1), Cell::parse_input(price));
        }
        let meta =
            excel_model::SheetMeta { tables: vec![Table::new("T", "A1:B6".parse().unwrap())], ..Default::default() };
        sheet.set_meta(meta);
        doc
    }

    fn run(doc: &mut Document, args: &[&str]) -> Result<String, CommandError> {
        let mut ctx = Context::new(doc, &NoCalculator, &NoStore);
        ctx.cursor = addr("A2");
        TableCommand.run(&mut ctx, args)?;
        Ok(ctx.message.unwrap_or_default())
    }

    /// The sheet rows (1-based) that show.
    fn shown(doc: &Document) -> Vec<u32> {
        (2..=6).filter(|row| !doc.workbook.active().is_row_hidden(row - 1)).collect()
    }

    #[test]
    fn filters_by_values_and_by_condition() {
        let mut d = doc();
        assert_eq!(
            run(&mut d, &["filter", "지역", "서울", "대구"]).unwrap(),
            "Filtered by 지역: 3 of 5 rows show in T"
        );
        assert_eq!(shown(&d), [2, 4, 5]);
        run(&mut d, &["filter", "지역", ""]).unwrap();
        assert_eq!(shown(&d), [6], "an empty condition shows blanks, replacing the earlier filter");
        run(&mut d, &["filter", "지역", "clear"]).unwrap();
        run(&mut d, &["filter", "가격", ">5000"]).unwrap();
        assert_eq!(shown(&d), [2, 5, 6]);
        run(&mut d, &["filter", "지역", "<>서울"]).unwrap();
        assert_eq!(shown(&d), [5, 6], "two columns combine with AND");
        assert_eq!(run(&mut d, &[]).unwrap(), "T A1:B6 (2 filters)");
        run(&mut d, &["filter", "지역", "clear"]).unwrap();
        assert_eq!(shown(&d), [2, 5, 6]);
        assert_eq!(run(&mut d, &["filter", "clear"]).unwrap(), "Cleared the filters: 5 of 5 rows show in T");
        assert_eq!(shown(&d), [2, 3, 4, 5, 6]);
    }

    #[test]
    fn hidden_rows_stay_until_filters_apply_again() {
        let mut d = doc();
        run(&mut d, &["filter", "지역", "서울"]).unwrap();
        assert_eq!(shown(&d), [2, 4]);
        // An edit makes row 3 match, and row 2 stop matching, but nothing moves yet.
        d.workbook.active_mut().set(addr("A3"), Cell::parse_input("서울"));
        d.workbook.active_mut().set(addr("A2"), Cell::parse_input("부산"));
        assert_eq!(shown(&d), [2, 4]);
        run(&mut d, &["sort", "가격"]).unwrap();
        // Sorted by price: 2000 서울, 3000 서울, 6000 blank, 7000 부산, 9000 대구.
        assert_eq!(shown(&d), [2, 3]);
    }

    #[test]
    fn undo_restores_filters_and_hidden_rows_together() {
        let mut d = doc();
        run(&mut d, &["filter", "가격", ">5000"]).unwrap();
        run(&mut d, &["filter", "clear"]).unwrap();
        d.undo();
        assert_eq!(shown(&d), [2, 5, 6]);
        assert_eq!(d.workbook.active().tables()[0].filters.len(), 1);
        d.undo();
        assert_eq!(shown(&d), [2, 3, 4, 5, 6]);
        assert!(d.workbook.active().tables()[0].filters.is_empty());
    }

    #[test]
    fn removing_a_table_shows_its_rows() {
        let mut d = doc();
        run(&mut d, &["filter", "가격", ">5000"]).unwrap();
        run(&mut d, &["remove"]).unwrap();
        assert_eq!(shown(&d), [2, 3, 4, 5, 6]);
    }

    #[test]
    fn filter_errors() {
        let mut d = doc();
        assert!(run(&mut d, &["filter"]).is_err());
        assert!(run(&mut d, &["filter", "지역"]).is_err());
        assert!(run(&mut d, &["filter", "무게", "1"]).unwrap_err().message.contains("no column 무게"));
    }
}
