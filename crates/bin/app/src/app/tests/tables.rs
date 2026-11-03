use super::*;

#[test]
fn ctrl_t_makes_a_table_and_typing_below_grows_it() {
    let mut h = blank();
    h.enter_cell("이름");
    h.enter_cell("사과");
    h.key_with(Key::T, Modifiers::COMMAND);
    assert!(h.app.doc.workbook.active().tables().is_empty(), "one cell is not a range");
    h.key(Key::ArrowUp);
    h.key(Key::ArrowUp);
    h.key_with(Key::ArrowDown, Modifiers::SHIFT);
    h.key_with(Key::T, Modifiers::COMMAND);
    let tables = |h: &Harness| -> Vec<String> {
        h.app.doc.workbook.active().tables().iter().map(|t| format!("{} {}", t.name, t.range)).collect()
    };
    assert_eq!(tables(&h), ["Table1 A1:A2"]);
    assert_eq!(h.app.focus, Focus::Grid, "the grid keeps the keyboard");
    h.key(Key::ArrowDown);
    h.enter_cell("배");
    assert_eq!(tables(&h), ["Table1 A1:A3"]);
    h.key_with(Key::Z, Modifiers::COMMAND);
    assert_eq!(tables(&h), ["Table1 A1:A2"], "one undo takes back the value and the growth");
    assert_eq!(h.value("A3"), CellValue::Empty);
}

#[test]
fn a_filtered_table_skips_hidden_rows_in_moves_stats_and_copies() {
    let mut h = blank();
    for text in ["값", "1", "20", "3", "40", "=SUM(A2:A5)"] {
        h.enter_cell(text);
    }
    h.drag(cell_center(0, 0), cell_center(5, 0));
    h.run_line("table create T");
    h.run_line("goto A2");
    h.run_line("table filter 값 >10");
    let sheet = h.app.doc.workbook.active();
    assert!(sheet.is_row_hidden(1) && sheet.is_row_hidden(3), "rows 2 and 4 are hidden");
    assert_eq!(h.value("A6"), CellValue::Number(64.0), "formulas still count hidden rows");
    assert_eq!(h.app.grid.cursor.to_string(), "A3", "the cursor left its hidden row");
    assert_eq!(
        excel_ui_status_bar::table_rows(sheet, h.app.grid.cursor).as_deref(),
        Some("3 of 5 rows"),
        "20, 40, and the sum pass"
    );

    h.key(Key::Escape);
    h.key(Key::ArrowDown);
    assert_eq!(h.app.grid.cursor.to_string(), "A5");
    h.key(Key::ArrowUp);
    h.key(Key::ArrowUp);
    assert_eq!(h.app.grid.cursor.to_string(), "A1");
    h.key_with(Key::ArrowDown, Modifiers::SHIFT);
    h.key_with(Key::ArrowDown, Modifiers::SHIFT);
    h.key_with(Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(h.app.grid.selection().to_string(), "A1:A6");
    let stats = excel_ui_status_bar::selection_stats(h.app.doc.workbook.active(), h.app.grid.selection());
    assert_eq!(stats.map(|s| s.sum), Some(124.0), "20 + 40 + 64, leaving out 1 and 3");
    let output = h.frame(vec![Event::Copy]);
    let copied = output.platform_output.commands.iter().find_map(|command| match command {
        egui::OutputCommand::CopyText(text) => Some(text.clone()),
        _ => None,
    });
    assert_eq!(copied.as_deref(), Some("값\n20\n40\n64"));
}

#[test]
fn the_header_popup_runs_the_same_table_commands() {
    let mut h = blank();
    for text in ["과일", "배", "사과", "감"] {
        h.enter_cell(text);
    }
    h.drag(cell_center(0, 0), cell_center(3, 0));
    h.run_line("table create");
    h.key(Key::Escape);
    click(&mut h, cell_center(0, 0) + vec2(40.0, 0.0));
    let popup = h.app.grid.filter_popup().expect("the button opens the popup").clone();
    h.key(Key::Escape);
    assert!(h.app.grid.filter_popup().is_none());
    let ctx = h.ctx.clone();
    h.app.apply_grid_action(&ctx, GridAction::RunCommand(popup.sort_line(true)));
    let fruits = |h: &Harness| -> Vec<String> { ["A2", "A3", "A4"].map(|at| h.value(at).to_string()).to_vec() };
    assert_eq!(fruits(&h), ["사과", "배", "감"]);
    let mut only = popup.clone();
    only.values.iter_mut().for_each(|value| value.1 = value.0 == "배");
    h.app.apply_grid_action(&ctx, GridAction::RunCommand(only.apply_line().unwrap()));
    let sheet = h.app.doc.workbook.active();
    assert_eq!((1..=3).filter(|row| !sheet.is_row_hidden(*row)).count(), 1);
    h.key_with(Key::Z, Modifiers::COMMAND);
    assert!(!h.app.doc.workbook.active().is_row_hidden(1), "undo works as for a typed command");
}

#[test]
fn enter_in_a_tables_last_row_adds_a_row() {
    let mut h = blank();
    h.enter_cell("이름");
    h.enter_cell("사과");
    h.run_line("set B1 가격");
    h.key(Key::Escape);
    h.drag(cell_center(0, 0), cell_center(1, 1));
    h.key_with(Key::T, Modifiers::COMMAND);
    let range = |h: &Harness| h.app.doc.workbook.active().tables()[0].range.to_string();
    assert_eq!(range(&h), "A1:B2");
    h.key(Key::Escape);
    h.key(Key::ArrowLeft);
    assert_eq!(h.app.grid.cursor.to_string(), "A2");
    h.key(Key::Tab);
    h.enter_cell("3");
    assert_eq!(range(&h), "A1:B3", "Enter after an edit in the last row adds one");
    assert_eq!(h.app.grid.cursor.to_string(), "A3", "and goes to its first column after the Tab");
    h.key(Key::ArrowUp);
    h.key(Key::Enter);
    assert_eq!(range(&h), "A1:B3", "moving with Enter without an edit adds nothing");
    h.key_with(Key::Z, Modifiers::COMMAND);
    assert_eq!(range(&h), "A1:B2", "one undo takes back the row and the edit");
    assert_eq!(h.value("B2"), CellValue::Empty);
}
