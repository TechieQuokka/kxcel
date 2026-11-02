use std::path::Path;

use excel_command::WorkbookStore;
use excel_model::{Cell, CellValue, Sheet, Workbook};
use excel_xlsx::{XlsxStore, inspect};
use rust_xlsxwriter::{Chart, ChartType, Format, Workbook as Writer};

fn warnings_for(build: impl FnOnce(&mut Writer)) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.xlsx");
    let mut writer = Writer::new();
    build(&mut writer);
    writer.save(&path).unwrap();
    inspect(&path).unwrap()
}

#[test]
fn plain_data_has_no_warnings() {
    let warnings = warnings_for(|writer| {
        let sheet = writer.add_worksheet();
        sheet.write_number(0, 0, 1.0).unwrap();
        sheet.write_string(0, 1, "x").unwrap();
        sheet.write_formula(0, 2, "=A1+1").unwrap();
    });
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn detects_cell_formatting() {
    let warnings = warnings_for(|writer| {
        let format = Format::new().set_bold();
        writer.add_worksheet().write_number_with_format(0, 0, 1.0, &format).unwrap();
    });
    assert_eq!(warnings, ["cell formatting"]);
}

#[test]
fn detects_charts() {
    let warnings = warnings_for(|writer| {
        let sheet = writer.add_worksheet();
        sheet.write_number(0, 0, 1.0).unwrap();
        let mut chart = Chart::new(ChartType::Column);
        chart.add_series().set_values("Sheet1!$A$1:$A$1");
        sheet.insert_chart(2, 2, &chart).unwrap();
    });
    assert!(warnings.contains(&"charts".to_owned()), "{warnings:?}");
}

#[test]
fn detects_merged_cells() {
    let warnings = warnings_for(|writer| {
        writer.add_worksheet().merge_range(0, 0, 1, 1, "x", &Format::new()).unwrap();
    });
    assert!(warnings.contains(&"merged cells".to_owned()), "{warnings:?}");
}

#[test]
fn loading_reports_warnings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.xlsx");
    let mut writer = Writer::new();
    writer.add_worksheet().write_number_with_format(0, 0, 1.0, &Format::new().set_bold()).unwrap();
    writer.save(&path).unwrap();
    assert_eq!(XlsxStore.load(&path).unwrap().loss_warnings, ["cell formatting"]);
}

#[test]
fn files_this_application_saves_have_no_warnings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.xlsx");
    let mut sheet = Sheet::new("Data");
    sheet.set("A1".parse().unwrap(), Some(Cell::literal(CellValue::Number(1.0))));
    sheet.set("A2".parse().unwrap(), Some(Cell::formula("A1+1", CellValue::Number(2.0))));
    sheet.set_col_width(0, 150.0);
    XlsxStore.save(&Workbook::from_sheets(vec![sheet]), &path).unwrap();
    assert!(inspect(Path::new(&path)).unwrap().is_empty());
}
