use excel_command::WorkbookStore;
use excel_model::{Cell, CellError, CellValue, Sheet, Workbook};
use excel_xlsx::XlsxStore;

fn round_trip(book: &Workbook) -> Workbook {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.xlsx");
    XlsxStore.save(book, &path).unwrap();
    XlsxStore.load(&path).unwrap().workbook
}

fn one_sheet(cells: Vec<(&str, Cell)>) -> Workbook {
    let mut sheet = Sheet::new("Data");
    for (addr, cell) in cells {
        sheet.set(addr.parse().unwrap(), Some(cell));
    }
    Workbook::from_sheets(vec![sheet])
}

#[test]
fn literals_survive() {
    let book = one_sheet(vec![
        ("A1", Cell::literal(CellValue::Number(1.5))),
        ("B2", Cell::literal(CellValue::Text("사과 & <pear>".into()))),
        ("C3", Cell::literal(CellValue::Bool(true))),
        ("D4", Cell::literal(CellValue::Number(1234567.891))),
    ]);
    assert_eq!(round_trip(&book), book);
}

#[test]
fn formulas_and_computed_values_survive() {
    for (formula, value) in [
        ("A1*2", CellValue::Number(3.25)),
        ("SUM(A1:A3)", CellValue::Number(0.1 + 0.2)),
        (r#"IF(A1>0,"예","no")"#, CellValue::Text("예".into())),
        ("A1>0", CellValue::Bool(true)),
        ("1/0", CellValue::Error(CellError::Div0)),
        ("$A$1+Data!B$2", CellValue::Number(-7.0)),
    ] {
        let book = one_sheet(vec![("C1", Cell::formula(formula, value.clone()))]);
        let loaded = round_trip(&book);
        let cell = loaded.active().get("C1".parse().unwrap()).unwrap();
        assert_eq!(cell.formula_text(), Some(formula));
        assert_eq!(cell.value, value, "{formula}");
    }
}

#[test]
fn unknown_functions_survive_with_their_stored_value() {
    let book = one_sheet(vec![("A1", Cell::formula("VLOOKUP(1,B1:C9,2,FALSE)", CellValue::Number(42.0)))]);
    assert_eq!(round_trip(&book), book);
}

#[test]
fn multiple_sheets_keep_their_order_and_names() {
    let book = Workbook::from_sheets(vec![Sheet::new("First"), Sheet::new("둘째"), Sheet::new("With Space")]);
    let names: Vec<String> = round_trip(&book).sheets().iter().map(|s| s.name.clone()).collect();
    assert_eq!(names, ["First", "둘째", "With Space"]);
}

#[test]
fn cells_far_from_the_origin_keep_their_address() {
    let book = one_sheet(vec![("XFD1048576", Cell::literal(CellValue::Number(1.0)))]);
    let loaded = round_trip(&book);
    assert_eq!(loaded.active().value("XFD1048576".parse().unwrap()), &CellValue::Number(1.0));
}

#[test]
fn an_empty_workbook_survives() {
    assert_eq!(round_trip(&Workbook::new()).sheets().len(), 1);
}

#[test]
fn saving_twice_gives_the_same_contents() {
    let book = one_sheet(vec![
        ("A1", Cell::literal(CellValue::Number(2.0))),
        ("A2", Cell::formula("A1+1", CellValue::Number(3.0))),
    ]);
    assert_eq!(round_trip(&round_trip(&book)), book);
}

#[test]
fn rejects_other_formats() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.csv");
    assert!(XlsxStore.save(&Workbook::new(), &path).is_err());
    std::fs::write(&path, "a,b").unwrap();
    assert!(XlsxStore.load(&path).is_err());
}

#[test]
fn reports_a_missing_file() {
    let error = XlsxStore.load(std::path::Path::new("/nonexistent/x.xlsx")).unwrap_err();
    assert!(error.0.contains("/nonexistent/x.xlsx"), "{error}");
}

#[test]
fn a_corrupt_file_is_an_error_and_saving_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.xlsx");
    std::fs::write(&path, "not a zip").unwrap();
    assert!(XlsxStore.load(&path).is_err());

    XlsxStore.save(&Workbook::new(), &path).unwrap();
    let names: Vec<_> = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(names, ["t.xlsx"]);
}

#[test]
fn an_invalid_sheet_name_fails_without_touching_the_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.xlsx");
    let good = one_sheet(vec![("A1", Cell::literal(CellValue::Number(1.0)))]);
    XlsxStore.save(&good, &path).unwrap();

    let bad = Workbook::from_sheets(vec![Sheet::new("a/b")]);
    assert!(XlsxStore.save(&bad, &path).is_err());
    assert_eq!(XlsxStore.load(&path).unwrap().workbook, good);
}

#[test]
fn dates_stay_dates_and_lose_nothing() {
    let book = one_sheet(vec![("A1", Cell::date(46_328.0)), ("B1", Cell::literal(CellValue::Number(46_328.0)))]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.xlsx");
    XlsxStore.save(&book, &path).unwrap();
    let loaded = XlsxStore.load(&path).unwrap();
    assert_eq!(loaded.workbook, book);
    assert_eq!(loaded.workbook.active().display("A1".parse().unwrap()), "2026-11-02");
    assert!(loaded.loss_warnings.is_empty(), "{:?}", loaded.loss_warnings);
}

#[test]
fn dates_from_other_programs_are_read_as_dates() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.xlsx");
    let mut writer = rust_xlsxwriter::Workbook::new();
    let sheet = writer.add_worksheet();
    let builtin = rust_xlsxwriter::Format::new().set_num_format_index(14);
    let korean = rust_xlsxwriter::Format::new().set_num_format("yyyy\"년\" m\"월\" d\"일\"");
    let with_time = rust_xlsxwriter::Format::new().set_num_format("yyyy-mm-dd hh:mm");
    sheet.write_number_with_format(0, 0, 46_328.0, &builtin).unwrap();
    sheet.write_number_with_format(1, 0, 46_329.0, &korean).unwrap();
    sheet.write_number_with_format(2, 0, 46_329.5, &with_time).unwrap();
    writer.save(&path).unwrap();

    let loaded = XlsxStore.load(&path).unwrap();
    let shown = |addr: &str| loaded.workbook.active().display(addr.parse().unwrap());
    assert_eq!(shown("A1"), "2026-11-02");
    assert_eq!(shown("A2"), "2026-11-03");
    assert_eq!(shown("A3"), "46329.5", "a time of day is not shown, so the number is");
    assert_eq!(loaded.loss_warnings, ["cell formatting"], "the time format would be lost");
}
