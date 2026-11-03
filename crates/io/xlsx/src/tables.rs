//! Tables, their filters, and hidden rows, read from and written into the
//! file's XML. `calamine` does not say where a table is, and `rust_xlsxwriter`
//! cannot write a table's filter conditions, so those parts are handled here.

use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Seek, Write};

use excel_model::{
    CellAddr, CellRange, ColumnFilter, FilterRule, MAX_COLS, MAX_ROWS, RangeSet, RowSet, SheetMeta, Table, Workbook,
    parse_date,
};
use zip::ZipArchive;
use zip::write::{SimpleFileOptions, ZipWriter};

use crate::xml::{attribute, elements, escape, raw_attribute, section, whole_elements};

type BoxError = Box<dyn std::error::Error>;

/// What the file holds besides cells, sheet by sheet, and anything about its
/// tables this application cannot keep.
#[derive(Debug, Default)]
pub(crate) struct FileMeta {
    pub sheets: HashMap<String, SheetMeta>,
    pub warnings: Vec<String>,
}

/// Reads tables, their filters, and hidden rows from every sheet.
pub(crate) fn read_meta<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<FileMeta, BoxError> {
    let mut found = FileMeta::default();
    let mut unsupported_filters = false;
    let mut headerless = false;
    let styles = read_part(archive, "xl/styles.xml").unwrap_or_default();
    let unlocked = unlocked_styles(&styles);
    let plain = plain_styles(&styles);
    for (name, part) in sheet_parts(archive)? {
        let Ok(xml) = read_part(archive, &part) else {
            continue;
        };
        let mut meta = SheetMeta {
            hidden_rows: hidden_rows(&xml),
            input_cells: styled_cells(&xml, &unlocked),
            plain_cells: styled_cells(&xml, &plain),
            ..Default::default()
        };
        for table_part in related(archive, &part, "/table")? {
            let Ok(table_xml) = read_part(archive, &table_part) else {
                continue;
            };
            match read_table(&table_xml) {
                Some((table, all_kept)) => {
                    unsupported_filters |= !all_kept;
                    meta.tables.push(table);
                }
                None => headerless = true,
            }
        }
        found.sheets.insert(name, meta);
    }
    if unsupported_filters {
        found.warnings.push("table filters other than values and simple conditions".to_owned());
    }
    if headerless {
        found.warnings.push("tables without a header row".to_owned());
    }
    Ok(found)
}

/// Each sheet's name and the path of its part, in workbook order.
fn sheet_parts<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<Vec<(String, String)>, BoxError> {
    let workbook = read_part(archive, "xl/workbook.xml")?;
    let rels = relationships(archive, "xl/_rels/workbook.xml.rels", "xl")?;
    Ok(elements(&workbook, "sheet")
        .filter_map(|sheet| {
            let name = attribute(sheet, "name")?;
            let id = raw_attribute(sheet, "r:id")?;
            let part = rels.iter().find(|(rel, _, _)| rel == id)?.1.clone();
            Some((name, part))
        })
        .collect())
}

/// The parts `part` refers to with a relationship type ending in `kind`.
fn related<R: Read + Seek>(archive: &mut ZipArchive<R>, part: &str, kind: &str) -> Result<Vec<String>, BoxError> {
    let (dir, file) = part.rsplit_once('/').unwrap_or(("", part));
    let rels_path = format!("{dir}/_rels/{file}.rels");
    if archive.index_for_name(&rels_path).is_none() {
        return Ok(Vec::new());
    }
    Ok(relationships(archive, &rels_path, dir)?
        .into_iter()
        .filter(|(_, _, kind_url)| kind_url.ends_with(kind))
        .map(|(_, target, _)| target)
        .collect())
}

/// Each relationship's id, target part path, and type. Targets are relative
/// to `dir` unless they start with `/`.
fn relationships<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    path: &str,
    dir: &str,
) -> Result<Vec<(String, String, String)>, BoxError> {
    let xml = read_part(archive, path)?;
    Ok(elements(&xml, "Relationship")
        .filter_map(|rel| {
            let id = attribute(rel, "Id")?;
            let target = attribute(rel, "Target")?;
            let kind = attribute(rel, "Type").unwrap_or_default();
            Some((id, join(dir, &target), kind))
        })
        .collect())
}

/// Resolves a relationship target against a folder, handling `..` and `/`.
fn join(dir: &str, target: &str) -> String {
    if let Some(absolute) = target.strip_prefix('/') {
        return absolute.to_owned();
    }
    let mut parts: Vec<&str> = dir.split('/').filter(|part| !part.is_empty()).collect();
    for piece in target.split('/') {
        match piece {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            piece => parts.push(piece),
        }
    }
    parts.join("/")
}

fn read_part<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Result<String, BoxError> {
    let mut text = String::new();
    archive.by_name(name)?.read_to_string(&mut text)?;
    Ok(text)
}

/// Rows marked hidden in a sheet's XML.
fn hidden_rows(sheet: &str) -> RowSet {
    elements(sheet, "row")
        .filter(|row| matches!(raw_attribute(row, "hidden"), Some("1" | "true")))
        .filter_map(|row| raw_attribute(row, "r")?.parse::<u32>().ok()?.checked_sub(1))
        .collect()
}

/// The cell styles, by index, that unlock their cells.
pub(crate) fn unlocked_styles(styles: &str) -> HashSet<String> {
    let Some(cell_xfs) = section(styles, "cellXfs") else {
        return HashSet::new();
    };
    whole_elements(cell_xfs, "xf")
        .iter()
        .enumerate()
        .filter(|(_, xf)| is_unlocked(xf))
        .map(|(index, _)| index.to_string())
        .collect()
}

/// The cell styles, by index, with a plain whole-number format such as `0`
/// or `0.00`: cells this application shows without commas.
pub(crate) fn plain_styles(styles: &str) -> HashSet<String> {
    let custom: HashSet<String> = elements(styles, "numFmt")
        .filter(|format| attribute(format, "formatCode").is_some_and(|code| is_plain_code(&code)))
        .filter_map(|format| attribute(format, "numFmtId"))
        .collect();
    let Some(cell_xfs) = section(styles, "cellXfs") else {
        return HashSet::new();
    };
    elements(cell_xfs, "xf")
        .enumerate()
        .filter(|(_, xf)| {
            raw_attribute(xf, "numFmtId").is_some_and(|id| matches!(id, "1" | "2") || custom.contains(id))
        })
        .map(|(index, _)| index.to_string())
        .collect()
}

/// `0`, `0.0`, `0.00`, and so on.
pub(crate) fn is_plain_code(code: &str) -> bool {
    let (whole, fraction) = code.split_once('.').unwrap_or((code, ""));
    whole == "0" && fraction.chars().all(|c| c == '0') && (fraction.is_empty() == !code.contains('.'))
}

/// `#,##0`, `#,##0.00`, and so on.
pub(crate) fn is_comma_code(code: &str) -> bool {
    let (whole, fraction) = code.split_once('.').unwrap_or((code, ""));
    whole == "#,##0" && fraction.chars().all(|c| c == '0') && (fraction.is_empty() == !code.contains('.'))
}

/// Whether a whole `xf` element unlocks its cells.
pub(crate) fn is_unlocked(xf: &str) -> bool {
    elements(xf, "protection").any(|protection| matches!(raw_attribute(protection, "locked"), Some("0" | "false")))
}

/// Cells and whole columns whose style is one of `styles`, such as the
/// unlocked input cells of an app.
fn styled_cells(sheet: &str, unlocked: &HashSet<String>) -> RangeSet {
    if unlocked.is_empty() {
        return RangeSet::default();
    }
    let mut ranges = Vec::new();
    for col in elements(sheet, "col").filter(|col| raw_attribute(col, "style").is_some_and(|s| unlocked.contains(s))) {
        let number = |name| raw_attribute(col, name).and_then(|n| n.parse::<u32>().ok()).and_then(|n| n.checked_sub(1));
        if let (Some(min), Some(max)) = (number("min"), number("max")) {
            ranges.push(CellRange {
                start: CellAddr::new(0, min),
                end: CellAddr::new(MAX_ROWS - 1, max.min(MAX_COLS - 1)),
            });
        }
    }
    // Cells come row by row, left to right, so runs along a row join first,
    // and then runs with the same columns in the rows below.
    let cells = elements(sheet, "c")
        .filter(|cell| raw_attribute(cell, "s").is_some_and(|style| unlocked.contains(style)))
        .filter_map(|cell| raw_attribute(cell, "r")?.parse::<CellAddr>().ok());
    let mut runs: Vec<CellRange> = Vec::new();
    for addr in cells {
        match runs.last_mut() {
            Some(run) if run.end.row == addr.row && run.end.col + 1 == addr.col => run.end.col = addr.col,
            _ => runs.push(CellRange::single(addr)),
        }
    }
    let mut open: HashMap<(u32, u32), usize> = HashMap::new();
    for run in runs {
        let key = (run.start.col, run.end.col);
        match open.get(&key).copied() {
            Some(index) if ranges[index].end.row + 1 == run.start.row => ranges[index].end.row = run.end.row,
            _ => {
                open.insert(key, ranges.len());
                ranges.push(run);
            }
        }
    }
    ranges.into_iter().collect()
}

/// Reads a table part. Returns the table and whether every filter in it was
/// kept, or `None` for a table without a header row, which cannot be kept.
fn read_table(xml: &str) -> Option<(Table, bool)> {
    let root = elements(xml, "table").next()?;
    if raw_attribute(root, "headerRowCount") == Some("0") {
        return None;
    }
    let name = attribute(root, "displayName").or_else(|| attribute(root, "name"))?;
    let mut range: CellRange = raw_attribute(root, "ref")?.parse().ok()?;
    let totals: u32 = raw_attribute(root, "totalsRowCount").and_then(|n| n.parse().ok()).unwrap_or(0);
    range.end.row = range.end.row.saturating_sub(totals).max(range.start.row);

    let mut table = Table::new(name, range);
    let mut all_kept = true;
    for column in section(xml, "autoFilter").map(|filter| whole_elements(filter, "filterColumn")).unwrap_or_default() {
        let col_id: Option<u32> = raw_attribute(column, "colId").and_then(|id| id.parse().ok());
        match (col_id, read_rule(column)) {
            (Some(id), Some(rule)) if id < range.cols() => {
                table.filters.push(ColumnFilter { column: range.start.col + id, rule });
            }
            _ => all_kept = false,
        }
    }
    Some((table, all_kept))
}

/// Reads one `filterColumn`: a list of values, or one custom condition.
fn read_rule(column: &str) -> Option<FilterRule> {
    if column.contains("<dateGroupItem") {
        return None;
    }
    if let Some(filters) = whole_elements(column, "filters").first() {
        let head = filters.split('>').next().unwrap_or_default();
        let mut values: Vec<String> =
            whole_elements(filters, "filter").iter().filter_map(|filter| attribute(filter, "val")).collect();
        if matches!(raw_attribute(head, "blank"), Some("1" | "true")) {
            values.push(String::new());
        }
        return match values.len() {
            0 => None,
            // One value alone would read as a condition, so it goes twice.
            1 => Some(FilterRule::Values(vec![values[0].clone(), values[0].clone()])),
            _ => Some(FilterRule::Values(values)),
        };
    }
    let customs = whole_elements(column, "customFilter");
    let [custom] = customs.as_slice() else {
        return None;
    };
    let value = attribute(custom, "val").unwrap_or_default();
    let prefix = match raw_attribute(custom, "operator").unwrap_or("equal") {
        "equal" => "=",
        "notEqual" if value == " " => return Some(FilterRule::Condition("<>".into())),
        "notEqual" => "<>",
        "lessThan" => "<",
        "lessThanOrEqual" => "<=",
        "greaterThan" => ">",
        "greaterThanOrEqual" => ">=",
        _ => return None,
    };
    Some(FilterRule::Condition(format!("{prefix}{value}")))
}

/// The `filterColumn` element for a filter on the table's column `col_id`.
pub(crate) fn filter_xml(col_id: u32, rule: &FilterRule) -> String {
    let inner = match rule {
        FilterRule::Values(values) => {
            let mut distinct: Vec<&str> = Vec::new();
            for value in values.iter().map(|value| value.trim()) {
                if !distinct.iter().any(|seen| seen.eq_ignore_ascii_case(value)) {
                    distinct.push(value);
                }
            }
            let blank = if distinct.contains(&"") { r#" blank="1""# } else { "" };
            let filters: String = distinct
                .iter()
                .filter(|value| !value.is_empty())
                .map(|value| format!(r#"<filter val="{}"/>"#, escape(value)))
                .collect();
            format!("<filters{blank}>{filters}</filters>")
        }
        FilterRule::Condition(condition) => condition_xml(condition),
    };
    format!(r#"<filterColumn colId="{col_id}">{inner}</filterColumn>"#)
}

/// A condition as Excel stores it. A date is written as its day number, which
/// is what Excel compares against.
fn condition_xml(condition: &str) -> String {
    let (operator, rest) = [
        ("<=", "lessThanOrEqual"),
        (">=", "greaterThanOrEqual"),
        ("<>", "notEqual"),
        ("<", "lessThan"),
        (">", "greaterThan"),
        ("=", "equal"),
    ]
    .into_iter()
    .find_map(|(prefix, operator)| condition.strip_prefix(prefix).map(|rest| (operator, rest)))
    .unwrap_or(("equal", condition));
    match (operator, rest) {
        ("equal", "") => r#"<filters blank="1"/>"#.to_owned(),
        ("notEqual", "") => r#"<customFilters><customFilter operator="notEqual" val=" "/></customFilters>"#.to_owned(),
        _ => {
            let value = parse_date(rest.trim()).map_or_else(|| rest.to_owned(), |serial| serial.to_string());
            let operator = if operator == "equal" { String::new() } else { format!(r#" operator="{operator}""#) };
            format!(r#"<customFilters><customFilter{operator} val="{}"/></customFilters>"#, escape(&value))
        }
    }
}

/// Adds every table's filters to a file `rust_xlsxwriter` wrote, which has
/// the tables and hidden rows but no filter conditions. Tables are found by
/// name. Returns the file unchanged when no table has filters.
pub(crate) fn add_filters(bytes: Vec<u8>, workbook: &Workbook) -> Result<Vec<u8>, BoxError> {
    let wanted: HashMap<String, String> = workbook
        .sheets()
        .iter()
        .flat_map(|sheet| sheet.tables())
        .filter(|table| !table.filters.is_empty())
        .map(|table| {
            let columns: String = table
                .filters
                .iter()
                .map(|filter| filter_xml(filter.column - table.range.start.col, &filter.rule))
                .collect();
            (table.name.to_lowercase(), columns)
        })
        .collect();
    if wanted.is_empty() {
        return Ok(bytes);
    }

    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    let mut output = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let name = archive.by_index(index)?.name()?.into_owned();
        if name.starts_with("xl/tables/") && name.ends_with(".xml") {
            let xml = read_part(&mut archive, &name)?;
            let root = elements(&xml, "table").next().unwrap_or_default();
            let table_name = attribute(root, "displayName").unwrap_or_default().to_lowercase();
            if let Some(columns) = wanted.get(&table_name) {
                output.start_file(name.as_str(), SimpleFileOptions::default())?;
                output.write_all(with_filter_columns(&xml, columns).as_bytes())?;
                continue;
            }
        }
        output.raw_copy_file(archive.by_index(index)?)?;
    }
    Ok(output.finish()?.into_inner())
}

/// Puts `columns` inside the table's `autoFilter` element.
fn with_filter_columns(xml: &str, columns: &str) -> String {
    let Some(start) = xml.find("<autoFilter") else {
        return xml.to_owned();
    };
    let head_end = start + xml[start..].find('>').unwrap_or(0);
    if xml[..head_end].ends_with('/') {
        let head = &xml[start..head_end - 1];
        format!("{}{}>{columns}</autoFilter>{}", &xml[..start], head.trim_end(), &xml[head_end + 1..])
    } else {
        format!("{}{columns}{}", &xml[..=head_end], &xml[head_end + 1..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(rule: FilterRule) -> Option<FilterRule> {
        read_rule(&filter_xml(0, &rule))
    }

    #[test]
    fn filters_round_trip_through_xml() {
        let values = FilterRule::Values(vec!["서울".into(), "a&b".into(), "".into()]);
        assert_eq!(round_trip(values.clone()), Some(values));
        for condition in [">5000", "<>서울", "사과*", "<=3", ">=1", "<1", "=x", "", "<>"] {
            let expected = match condition {
                "" => FilterRule::Values(vec![String::new(), String::new()]),
                "=x" => FilterRule::Condition("=x".into()),
                other => FilterRule::Condition(other.into()),
            };
            let expected = if condition == "사과*" { FilterRule::Condition("=사과*".into()) } else { expected };
            assert_eq!(round_trip(FilterRule::Condition(condition.into())), Some(expected), "{condition}");
        }
    }

    #[test]
    fn dates_are_written_as_day_numbers() {
        assert!(filter_xml(1, &FilterRule::Condition(">=2026-01-01".into())).contains(r#"val="46023""#));
    }

    #[test]
    fn reads_a_table_part() {
        let xml = r#"<?xml version="1.0"?><table xmlns="x" id="1" name="T" displayName="매출" ref="B2:D10" totalsRowCount="1">
            <autoFilter ref="B2:D9"><filterColumn colId="2"><filters><filter val="서울"/></filters></filterColumn>
            <filterColumn colId="0"><top10 val="5"/></filterColumn></autoFilter><tableColumns count="3"/></table>"#;
        let (table, all_kept) = read_table(xml).unwrap();
        assert_eq!(table.name, "매출");
        assert_eq!(table.range.to_string(), "B2:D9", "the totals row is left out");
        assert_eq!(table.filters.len(), 1);
        assert_eq!(table.filters[0].column, 3);
        assert!(!all_kept, "a top 10 filter cannot be kept");
        assert!(read_table(r#"<table displayName="T" ref="A1:B2" headerRowCount="0"/>"#).is_none());
    }

    #[test]
    fn reads_hidden_rows() {
        let xml = r#"<row r="2" hidden="1"><c r="A2"/></row><row r="3"/><row r="5" hidden="true"/>"#;
        assert_eq!(hidden_rows(xml).iter().collect::<Vec<_>>(), [1, 4]);
    }

    #[test]
    fn puts_columns_inside_auto_filters() {
        assert_eq!(
            with_filter_columns(r#"<a><autoFilter ref="A1:B2"/></a>"#, "<x/>"),
            r#"<a><autoFilter ref="A1:B2"><x/></autoFilter></a>"#
        );
        assert_eq!(
            with_filter_columns(r#"<autoFilter ref="A1"></autoFilter>"#, "<x/>"),
            r#"<autoFilter ref="A1"><x/></autoFilter>"#
        );
    }

    #[test]
    fn reads_unlocked_cells_and_columns() {
        let styles = r#"<cellXfs count="3"><xf numFmtId="0"/><xf numFmtId="0" applyProtection="1"><protection locked="0"/></xf>
            <xf numFmtId="14"><protection locked="1"/></xf></cellXfs>"#;
        let unlocked = unlocked_styles(styles);
        assert_eq!(unlocked, HashSet::from(["1".to_owned()]));
        let sheet = r#"<cols><col min="5" max="5" style="1"/></cols><sheetData><row r="2"><c r="B2" s="1"/><c r="C2" s="1"><v>1</v></c><c r="D2" s="2"/></row>
            <row r="3"><c r="B3" s="1"/><c r="C3" s="1"/></row></sheetData>"#;
        let ranges: Vec<String> = styled_cells(sheet, &unlocked).ranges().iter().map(|r| r.to_string()).collect();
        assert_eq!(ranges, ["E1:E1048576", "B2:C3"]);
    }

    #[test]
    fn recognizes_plain_and_comma_codes() {
        for code in ["0", "0.00"] {
            assert!(is_plain_code(code) && !is_comma_code(code), "{code}");
        }
        for code in ["#,##0", "#,##0.0"] {
            assert!(is_comma_code(code) && !is_plain_code(code), "{code}");
        }
        for code in ["0.", "00", "#,##0.#", "General", "0%"] {
            assert!(!is_plain_code(code) && !is_comma_code(code), "{code}");
        }
        let styles = r#"<numFmts><numFmt numFmtId="164" formatCode="0.000"/></numFmts>
            <cellXfs><xf numFmtId="0"/><xf numFmtId="1"/><xf numFmtId="164"/><xf numFmtId="3"/></cellXfs>"#;
        let mut plain: Vec<String> = plain_styles(styles).into_iter().collect();
        plain.sort();
        assert_eq!(plain, ["1", "2"]);
    }

    #[test]
    fn joins_relationship_targets() {
        assert_eq!(join("xl/worksheets", "../tables/table1.xml"), "xl/tables/table1.xml");
        assert_eq!(join("xl", "worksheets/sheet1.xml"), "xl/worksheets/sheet1.xml");
        assert_eq!(join("xl", "/xl/worksheets/sheet2.xml"), "xl/worksheets/sheet2.xml");
    }
}
