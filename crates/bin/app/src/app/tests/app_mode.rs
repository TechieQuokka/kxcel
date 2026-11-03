use super::*;

#[test]
fn app_mode_changes_only_input_cells_and_has_no_commands() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = app_harness(dir.path(), false);
    h.enter_cell("7");
    assert_eq!(h.value("A1"), CellValue::Empty, "A1 is not an input cell");
    h.key(Key::Tab);
    assert_eq!(h.app.grid.cursor.to_string(), "B2", "Tab goes to the input cell");
    h.enter_cell("3");
    assert_eq!(h.value("C2"), CellValue::Number(3000.0));
    h.key_with(Key::L, Modifiers::COMMAND);
    assert_eq!(h.app.focus, Focus::Grid, "no command line");
    h.key(Key::F1);
    assert!(!h.app.help.is_open(), "no help");
    h.key(Key::ArrowLeft);
    assert_eq!(h.app.grid.cursor.to_string(), "A2");
    h.key(Key::Delete);
    assert_eq!(h.value("A2"), CellValue::Text("수량".into()), "nothing outside input cells clears");
    assert_eq!(h.app.app_content(), "수량", "the status bar shows values");
    let ctx = h.ctx.clone();
    h.app.apply_grid_action(&ctx, GridAction::RunCommand("set A2 x".into()));
    assert_eq!(h.value("A2"), CellValue::Text("수량".into()), "commands do not run");
}

#[test]
fn an_app_that_saves_saves_and_asks_on_close() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, path) = app_harness(dir.path(), false);
    assert_eq!(h.app.title, "견적");
    h.key(Key::Tab);
    h.enter_cell("4");
    assert_eq!(h.app.title, "견적 *");
    h.key_with(Key::S, Modifiers::COMMAND);
    assert!(!h.app.doc.dirty);
    let saved = excel_features::Services::default().open(&path).unwrap();
    assert_eq!(saved.workbook.active().value("B2".parse().unwrap()), &CellValue::Number(4.0));
    h.key(Key::Tab);
    h.enter_cell("5");
    let commands = close_window(&mut h);
    assert!(commands.contains(&ViewportCommand::CancelClose));
    assert!(matches!(h.app.dialog, Some(Dialog::Confirm { .. })));
}

#[test]
fn a_nosave_app_never_saves_or_asks() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, path) = app_harness(dir.path(), true);
    let before = std::fs::read(&path).unwrap();
    h.key(Key::Tab);
    h.enter_cell("9");
    assert_eq!(h.app.title, "견적", "nothing to mark");
    h.key_with(Key::S, Modifiers::COMMAND);
    assert_eq!(std::fs::read(&path).unwrap(), before, "the file is untouched");
    assert_eq!(h.app.app_content(), "This app does not save");
    let commands = close_window(&mut h);
    assert!(!commands.contains(&ViewportCommand::CancelClose));
    assert!(h.app.dialog.is_none());
}
