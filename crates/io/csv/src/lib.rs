//! Reads and writes `.csv` files.
//!
//! A CSV file holds one sheet of values. Reading gives a workbook with one
//! sheet named after the file. Writing saves the active sheet's values, so
//! formulas keep only their results and other sheets are left out. The store
//! reports both through [`WorkbookStore::save_losses`] so the user can confirm.
//!
//! Files are UTF-8. A byte order mark, which Excel writes, is skipped on reading,
//! and none is written.

use std::fs;
use std::path::{Path, PathBuf};

use excel_command::{LoadedWorkbook, StoreError, WorkbookStore};
use excel_model::{Cell, CellAddr, CellValue, Sheet, Workbook};

/// Stores workbooks as `.csv` files.
#[derive(Debug, Default, Clone, Copy)]
pub struct CsvStore;

impl WorkbookStore for CsvStore {
    fn load(&self, path: &Path) -> Result<LoadedWorkbook, StoreError> {
        let bytes = fs::read(path).map_err(|e| file_error("open", path, e))?;
        let sheet = parse(&bytes, sheet_name(path)).map_err(|e| file_error("open", path, e))?;
        Ok(LoadedWorkbook { workbook: Workbook::from_sheets(vec![sheet]), loss_warnings: Vec::new() })
    }

    fn save(&self, workbook: &Workbook, path: &Path) -> Result<(), StoreError> {
        let bytes = encode(workbook.active()).map_err(|e| file_error("save", path, e))?;
        let temporary = temporary_path(path);
        fs::write(&temporary, bytes).map_err(|e| file_error("save", path, e))?;
        fs::rename(&temporary, path).map_err(|e| {
            let _ = fs::remove_file(&temporary);
            file_error("save", path, e)
        })
    }

    fn save_losses(&self, workbook: &Workbook, _path: &Path) -> Vec<String> {
        let mut losses = Vec::new();
        if workbook.active().iter().any(|(_, cell)| cell.formula_text().is_some()) {
            losses.push("formulas (their values are kept)".to_owned());
        }
        if workbook.sheets().len() > 1 {
            losses.push("other sheets".to_owned());
        }
        losses
    }
}

/// Reads CSV bytes into a sheet. Fields become numbers, `TRUE` and `FALSE`,
/// error values, or text. Text starting with `=` stays text, since a CSV file
/// holds values only.
fn parse(bytes: &[u8], name: String) -> Result<Sheet, csv::Error> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let mut reader = csv::ReaderBuilder::new().has_headers(false).flexible(true).from_reader(bytes);
    let mut sheet = Sheet::new(name);
    for (row, record) in reader.records().enumerate() {
        for (col, field) in record?.iter().enumerate() {
            let cell = match Cell::parse_input(field) {
                Some(cell) if cell.formula_text().is_some() => Some(Cell::literal(CellValue::Text(field.to_owned()))),
                cell => cell,
            };
            sheet.set(CellAddr::new(row as u32, col as u32), cell);
        }
    }
    Ok(sheet)
}

/// Writes a sheet's values from A1 to its last used cell.
fn encode(sheet: &Sheet) -> Result<Vec<u8>, csv::Error> {
    let mut writer = csv::WriterBuilder::new().flexible(true).from_writer(Vec::new());
    if let Some(used) = sheet.used_range() {
        for row in 0..=used.end.row {
            let record: Vec<String> = (0..=used.end.col).map(|col| sheet.display(CellAddr::new(row, col))).collect();
            writer.write_record(&record)?;
        }
    }
    writer.into_inner().map_err(|e| e.into_error().into())
}

/// The file name without its extension, made valid as an `.xlsx` sheet name.
fn sheet_name(path: &Path) -> String {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let clean: String = stem
        .chars()
        .map(|c| if matches!(c, '[' | ']' | ':' | '*' | '?' | '/' | '\\') { '_' } else { c })
        .take(31)
        .collect();
    let clean = clean.trim_matches('\'').to_owned();
    if clean.is_empty() { "Sheet1".to_owned() } else { clean }
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

fn file_error(action: &str, path: &Path, cause: impl std::fmt::Display) -> StoreError {
    StoreError(format!("Cannot {action} {}: {cause}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(text: &str) -> Sheet {
        parse(text.as_bytes(), "S".into()).unwrap()
    }

    fn at(sheet: &Sheet, addr: &str) -> CellValue {
        sheet.value(addr.parse().unwrap()).clone()
    }

    #[test]
    fn reads_values_by_type() {
        let s = sheet("이름,수량,ok\n사과,3,TRUE\n\"배, 큰 것\",-1.5,#N/A\n=1+1,,\n");
        assert_eq!(at(&s, "A1"), CellValue::Text("이름".into()));
        assert_eq!(at(&s, "B2"), CellValue::Number(3.0));
        assert_eq!(at(&s, "C2"), CellValue::Bool(true));
        assert_eq!(at(&s, "A3"), CellValue::Text("배, 큰 것".into()));
        assert_eq!(at(&s, "B3"), CellValue::Number(-1.5));
        assert_eq!(at(&s, "C3"), CellValue::Error(excel_model::CellError::Na));
        assert_eq!(at(&s, "A4"), CellValue::Text("=1+1".into()), "formulas stay text");
        assert_eq!(at(&s, "B4"), CellValue::Empty);
    }

    #[test]
    fn skips_a_byte_order_mark_and_allows_ragged_rows() {
        let s = parse(b"\xEF\xBB\xBFa,b,c\n1\n", "S".into()).unwrap();
        assert_eq!(at(&s, "A1"), CellValue::Text("a".into()));
        assert_eq!(at(&s, "A2"), CellValue::Number(1.0));
    }

    #[test]
    fn writes_values_and_quotes_when_needed() {
        let mut s = Sheet::new("S");
        s.set("A1".parse().unwrap(), Cell::parse_input("a,b"));
        s.set("B1".parse().unwrap(), Some(Cell::formula("1+1", CellValue::Number(2.0))));
        s.set("A2".parse().unwrap(), Cell::parse_input("line\nbreak"));
        s.set("C3".parse().unwrap(), Cell::parse_input("0.1"));
        let text = String::from_utf8(encode(&s).unwrap()).unwrap();
        assert_eq!(text, "\"a,b\",2,\n\"line\nbreak\",,\n,,0.1\n");
    }

    #[test]
    fn dates_are_written_as_dates_and_read_back() {
        let mut s = Sheet::new("S");
        s.set("A1".parse().unwrap(), Some(Cell::date(46_328.0)));
        let text = String::from_utf8(encode(&s).unwrap()).unwrap();
        assert_eq!(text, "2026-11-02\n");
        let read = sheet(&text);
        assert_eq!(read.get("A1".parse().unwrap()), Some(&Cell::date(46_328.0)));
    }

    #[test]
    fn names_the_sheet_after_the_file() {
        assert_eq!(sheet_name(Path::new("/x/매출 [2026].csv")), "매출 _2026_");
        assert_eq!(sheet_name(Path::new(&format!("{}.csv", "a".repeat(40)))).len(), 31);
    }
}
