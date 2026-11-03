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
fn the_active_sheet_survives() {
    for active in 0..3 {
        let mut book = Workbook::from_sheets(vec![Sheet::new("First"), Sheet::new("견적"), Sheet::new("Third")]);
        assert!(book.set_active(active));
        assert_eq!(round_trip(&book).active_index(), active);
    }
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

fn table_book() -> Workbook {
    use excel_model::{ColumnFilter, FilterRule, SheetMeta, Table};
    let mut sheet = Sheet::new("Data");
    let rows = [["지역", "가격"], ["서울", "7000"], ["부산", "3000"], ["서울", "2000"], ["대구", "9000"]];
    for (r, row) in rows.iter().enumerate() {
        for (c, text) in row.iter().enumerate() {
            sheet.set(excel_model::CellAddr::new(r as u32 + 1, c as u32 + 1), Cell::parse_input(text));
        }
    }
    let mut table = Table::new("매출", "B2:C6".parse().unwrap());
    table.filters = vec![
        ColumnFilter { column: 1, rule: FilterRule::Values(vec!["서울".into(), "대구".into()]) },
        ColumnFilter { column: 2, rule: FilterRule::Condition(">5000".into()) },
    ];
    let mut meta = SheetMeta { tables: vec![table], ..Default::default() };
    meta.hidden_rows = sheet.rows_hidden_by(&meta.tables);
    sheet.set_meta(meta);
    Workbook::from_sheets(vec![sheet])
}

#[test]
fn tables_filters_and_hidden_rows_survive() {
    let book = table_book();
    assert_eq!(book.active().hidden_rows().iter().collect::<Vec<_>>(), [3, 4], "부산 and the cheap 서울");
    assert_eq!(round_trip(&book), book);
}

#[test]
fn a_saved_table_opens_in_calamine_and_warns_of_nothing() {
    use calamine::{Reader, Xlsx, open_workbook};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.xlsx");
    XlsxStore.save(&table_book(), &path).unwrap();
    let mut file: Xlsx<_> = open_workbook(&path).unwrap();
    file.load_tables().unwrap();
    let table = file.table_by_name("매출").unwrap();
    assert_eq!(table.columns(), ["지역", "가격"]);
    assert!(file.worksheet_range("Data").is_ok());
    assert_eq!(XlsxStore.load(&path).unwrap().loss_warnings, Vec::<String>::new());
}

#[test]
fn a_table_from_another_writer_opens() {
    use rust_xlsxwriter::{Table as XlsxTable, Workbook as Writer};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("other.xlsx");
    let mut writer = Writer::new();
    let sheet = writer.add_worksheet();
    sheet.write_string(0, 0, "Name").unwrap();
    sheet.write_string(0, 1, "Size").unwrap();
    sheet.write_string(1, 0, "a").unwrap();
    sheet.write_number(1, 1, 3.0).unwrap();
    sheet.write_string(2, 0, "b").unwrap();
    sheet.set_row_hidden(2).unwrap();
    sheet.add_table(0, 0, 2, 1, &XlsxTable::new().set_name("Sizes")).unwrap();
    writer.save(&path).unwrap();
    let loaded = XlsxStore.load(&path).unwrap();
    let sheet = loaded.workbook.active();
    assert_eq!(sheet.tables().len(), 1);
    assert_eq!(sheet.tables()[0].name, "Sizes");
    assert_eq!(sheet.tables()[0].range.to_string(), "A1:B3");
    assert!(sheet.is_row_hidden(2));
    assert!(loaded.loss_warnings.is_empty(), "{:?}", loaded.loss_warnings);
}

#[test]
fn a_table_that_is_only_a_header_gains_a_row() {
    use excel_model::{SheetMeta, Table};
    let mut sheet = Sheet::new("S");
    sheet.set("A1".parse().unwrap(), Cell::parse_input("h"));
    sheet.set_meta(SheetMeta { tables: vec![Table::new("T", "A1".parse().unwrap())], ..Default::default() });
    let loaded = round_trip(&Workbook::from_sheets(vec![sheet]));
    assert_eq!(loaded.active().tables()[0].range.to_string(), "A1:A2");
}

#[test]
fn input_cells_survive_with_empty_cells_dates_and_whole_columns() {
    let mut sheet = Sheet::new("Form");
    sheet.set("B2".parse().unwrap(), Cell::parse_input("2026-03-01"));
    sheet.set("B3".parse().unwrap(), Cell::parse_input("=B2+1"));
    sheet.set("C2".parse().unwrap(), Cell::parse_input("고정"));
    let mut meta = sheet.meta().clone();
    meta.input_cells.add("B2:B5".parse().unwrap());
    meta.input_cells.add("E1:E1048576".parse().unwrap());
    sheet.set_meta(meta);
    let book = Workbook::from_sheets(vec![sheet]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("form.xlsx");
    XlsxStore.save(&book, &path).unwrap();
    let loaded = XlsxStore.load(&path).unwrap();
    assert_eq!(loaded.workbook.active().meta(), book.active().meta());
    assert_eq!(loaded.workbook.active().len(), 3, "empty input cells add no cells");
    assert!(loaded.loss_warnings.is_empty(), "{:?}", loaded.loss_warnings);
    let b2 = loaded.workbook.active().get("B2".parse().unwrap()).unwrap();
    assert_eq!(b2.display(), "2026-03-01", "a date stays a date");
}

#[test]
fn unlocked_cells_from_another_writer_are_input_cells() {
    use rust_xlsxwriter::{Format, Workbook as Writer};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("other.xlsx");
    let mut writer = Writer::new();
    let unlocked = Format::new().set_unlocked();
    let sheet = writer.add_worksheet();
    sheet.write_number_with_format(0, 0, 5.0, &unlocked).unwrap();
    sheet.write_blank(1, 0, &unlocked).unwrap();
    sheet.write_number(0, 1, 1.0).unwrap();
    sheet.protect();
    writer.save(&path).unwrap();
    let loaded = XlsxStore.load(&path).unwrap();
    let ranges: Vec<String> =
        loaded.workbook.active().meta().input_cells.ranges().iter().map(|r| r.to_string()).collect();
    assert_eq!(ranges, ["A1:A2"]);
    assert!(!loaded.loss_warnings.contains(&"cell formatting".to_owned()), "{:?}", loaded.loss_warnings);
}

#[test]
fn plain_cells_and_comma_numbers_survive() {
    let mut sheet = Sheet::new("Years");
    for (at, text) in [("A1", "2026"), ("A2", "1999.5"), ("B1", "1234567"), ("B2", "1234.25"), ("C3", "2025")] {
        sheet.set(at.parse().unwrap(), Cell::parse_input(text));
    }
    let mut meta = sheet.meta().clone();
    meta.plain_cells.add("A1:A1048576".parse().unwrap());
    meta.plain_cells.add("C2:C4".parse().unwrap());
    sheet.set_meta(meta);
    let book = Workbook::from_sheets(vec![sheet]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("years.xlsx");
    XlsxStore.save(&book, &path).unwrap();
    let loaded = XlsxStore.load(&path).unwrap();
    assert_eq!(loaded.workbook, book);
    assert!(loaded.loss_warnings.is_empty(), "{:?}", loaded.loss_warnings);
    let sheet = loaded.workbook.active();
    let shown = |at: &str| sheet.shown(at.parse().unwrap());
    assert_eq!(
        [shown("A1"), shown("A2"), shown("B1"), shown("B2"), shown("C3")],
        ["2026", "1999.5", "1,234,567", "1,234.25", "2025"]
    );
}
