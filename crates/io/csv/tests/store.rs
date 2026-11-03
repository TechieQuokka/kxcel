//! `CsvStore` against real files in a temporary folder.

use std::fs;
use std::path::Path;

use excel_command::WorkbookStore;
use excel_csv::CsvStore;
use excel_model::{Cell, CellAddr, CellRange, CellValue, Sheet, SheetMeta, Table, Workbook};

fn addr(text: &str) -> CellAddr {
    text.parse().unwrap()
}

fn sheet_with(name: &str, cells: &[(&str, &str)]) -> Sheet {
    let mut sheet = Sheet::new(name);
    for (at, input) in cells {
        sheet.set(addr(at), Cell::parse_input(input));
    }
    sheet
}

fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

#[test]
fn saving_and_opening_gives_the_same_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("목록.csv");
    let sheet = sheet_with(
        "S",
        &[
            ("A1", "이름"),
            ("B1", "수량"),
            ("A2", "사과, 큰 것"),
            ("B2", "3"),
            ("C2", "TRUE"),
            ("A3", "줄\n바꿈"),
            ("B3", "-1.5"),
        ],
    );
    CsvStore.save(&Workbook::from_sheets(vec![sheet]), &path).unwrap();

    let loaded = CsvStore.load(&path).unwrap();
    assert!(loaded.loss_warnings.is_empty());
    let read = loaded.workbook.active();
    assert_eq!(read.name, "목록");
    assert_eq!(read.value(addr("A1")), &CellValue::Text("이름".into()));
    assert_eq!(read.value(addr("A2")), &CellValue::Text("사과, 큰 것".into()));
    assert_eq!(read.value(addr("B2")), &CellValue::Number(3.0));
    assert_eq!(read.value(addr("C2")), &CellValue::Bool(true));
    assert_eq!(read.value(addr("A3")), &CellValue::Text("줄\n바꿈".into()));
    assert_eq!(read.value(addr("B3")), &CellValue::Number(-1.5));
}

#[test]
fn large_numbers_are_saved_without_commas() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("n.csv");
    let sheet = sheet_with("S", &[("A1", "1234567.5"), ("B1", "-1,000")]);
    CsvStore.save(&Workbook::from_sheets(vec![sheet]), &path).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "1234567.5,-1000\n");
}

#[test]
fn overwriting_replaces_the_file_and_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.csv");
    fs::write(&path, "old,content\nmore\n").unwrap();
    CsvStore.save(&Workbook::from_sheets(vec![sheet_with("S", &[("A1", "new")])]), &path).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
    assert_eq!(files_in(dir.path()), ["out.csv"]);
}

#[test]
fn only_the_active_sheet_is_saved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("two.csv");
    let mut workbook =
        Workbook::from_sheets(vec![sheet_with("First", &[("A1", "first")]), sheet_with("Second", &[("A1", "second")])]);
    assert!(workbook.set_active(1));
    CsvStore.save(&workbook, &path).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "second\n");
}

#[test]
fn hidden_rows_are_still_saved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hidden.csv");
    let mut sheet = sheet_with("S", &[("A1", "a"), ("A2", "b"), ("A3", "c")]);
    let mut meta = SheetMeta::default();
    meta.hidden_rows.insert(1);
    sheet.set_meta(meta);
    assert!(sheet.is_row_hidden(1));
    CsvStore.save(&Workbook::from_sheets(vec![sheet]), &path).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "a\nb\nc\n");
}

#[test]
fn an_empty_sheet_saves_an_empty_file_that_opens_blank() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.csv");
    CsvStore.save(&Workbook::from_sheets(vec![Sheet::new("S")]), &path).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"");
    let loaded = CsvStore.load(&path).unwrap();
    assert!(loaded.workbook.active().is_empty());
    assert_eq!(loaded.workbook.sheets().len(), 1);
}

#[test]
fn opening_a_missing_file_says_which_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("none.csv");
    let Err(error) = CsvStore.load(&path) else { panic!("a missing file fails") };
    assert!(error.0.starts_with("Cannot open "), "{}", error.0);
    assert!(error.0.contains("none.csv"), "{}", error.0);
}

#[test]
fn saving_into_a_missing_folder_fails_and_leaves_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing").join("out.csv");
    let workbook = Workbook::from_sheets(vec![sheet_with("S", &[("A1", "x")])]);
    let error = CsvStore.save(&workbook, &path).unwrap_err();
    assert!(error.0.starts_with("Cannot save "), "{}", error.0);
    assert!(files_in(dir.path()).is_empty());
}

#[test]
fn save_losses_lists_what_a_csv_file_cannot_keep() {
    let path = Path::new("out.csv");
    let plain = Workbook::from_sheets(vec![sheet_with("S", &[("A1", "1"), ("A2", "text")])]);
    assert!(CsvStore.save_losses(&plain, path).is_empty());

    let mut sheet = sheet_with("S", &[("A1", "1")]);
    sheet.set(addr("A2"), Some(Cell::formula("A1+1", CellValue::Number(2.0))));
    let mut meta = SheetMeta::default();
    meta.tables.push(Table::new("Table1", CellRange::new(addr("A1"), addr("A2"))));
    meta.input_cells.add(CellRange::new(addr("B1"), addr("B1")));
    sheet.set_meta(meta);
    let everything = Workbook::from_sheets(vec![sheet, Sheet::new("Other")]);
    assert_eq!(
        CsvStore.save_losses(&everything, path),
        [
            "formulas (their values are kept)",
            "other sheets",
            "tables and their filters (hidden rows are still written)",
            "input cells",
        ]
    );
}
