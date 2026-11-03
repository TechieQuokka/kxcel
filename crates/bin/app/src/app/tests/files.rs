use super::*;

#[test]
fn multi_line_cells_survive_saving() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.xlsx");
    let mut h = Harness::new(ExcelApp::new(Some(path.clone())));
    h.typed("a");
    h.key_with(Key::Enter, Modifiers::ALT);
    h.typed("b");
    h.key(Key::Enter);
    h.key_with(Key::S, Modifiers::COMMAND);
    let reopened = Harness::new(ExcelApp::new(Some(path)));
    assert_eq!(reopened.value("A1"), CellValue::Text("a\nb".into()));
}

#[test]
fn ctrl_s_saves_and_exit_asks_about_unsaved_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.xlsx");
    let mut h = Harness::new(ExcelApp::new(Some(path.clone())));
    h.enter_cell("1");
    h.run_line("exit");
    assert!(!h.app.allow_close, "unsaved changes block exiting");
    assert!(matches!(h.app.dialog, Some(Dialog::Confirm { .. })));
    h.key(Key::Escape);
    assert!(h.app.dialog.is_none() && !h.app.allow_close, "escape cancels");

    h.key_with(Key::S, Modifiers::COMMAND);
    assert!(path.exists());
    assert!(!h.app.doc.dirty);
    h.run_line("exit");
    assert!(h.app.allow_close);
}

#[test]
fn the_exit_dialog_can_save_first_or_discard() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.xlsx");
    let mut h = Harness::new(ExcelApp::new(Some(path.clone())));
    h.enter_cell("1");
    h.run_line("exit");
    answer(&mut h, dialogs::Choice::Save);
    assert!(path.exists() && h.app.allow_close);

    let mut h = blank();
    h.enter_cell("1");
    h.run_line("exit");
    answer(&mut h, dialogs::Choice::Proceed);
    assert!(h.app.allow_close, "don't save exits");
}

#[test]
fn closing_the_window_asks_like_exit() {
    let mut h = blank();
    h.enter_cell("1");
    h.time += 0.1;
    let mut input = RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), WINDOW)),
        time: Some(h.time),
        ..Default::default()
    };
    let info = egui::ViewportInfo { events: vec![egui::ViewportEvent::Close], ..Default::default() };
    input.viewports.insert(egui::ViewportId::ROOT, info);
    let app = &mut h.app;
    let mut output = h.ctx.run_ui(input, |ui| app.show(ui));
    output.textures_delta.clear();
    let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
    assert!(commands.contains(&ViewportCommand::CancelClose));
    assert!(matches!(
        &h.app.dialog,
        Some(Dialog::Confirm { confirmation: excel_command::Confirmation::UnsavedChanges { action, .. }, .. })
            if action == "closing"
    ));
}

#[test]
fn open_asks_about_unsaved_changes_then_opens() {
    let dir = tempfile::tempdir().unwrap();
    let other = dir.path().join("other file.xlsx");
    rust_xlsxwriter::Workbook::new().save(&other).unwrap();
    let mut h = blank();
    h.enter_cell("1");
    h.run_line(&format!("open \"{}\"", other.display()));
    assert!(matches!(h.app.dialog, Some(Dialog::Confirm { .. })));
    assert_eq!(h.value("A1"), CellValue::Number(1.0), "nothing happens before an answer");
    answer(&mut h, dialogs::Choice::Proceed);
    assert_eq!(h.app.doc.path.as_deref(), Some(other.as_path()));
    assert_eq!(h.value("A1"), CellValue::Empty);
    assert_eq!(h.app.focus, Focus::CommandLine);
}

#[test]
fn saving_over_lost_content_asks_and_save_first_chains_through_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("styled.xlsx");
    let mut writer = rust_xlsxwriter::Workbook::new();
    let bold = rust_xlsxwriter::Format::new().set_bold();
    writer.add_worksheet().write_number_with_format(0, 0, 1.0, &bold).unwrap();
    writer.save(&path).unwrap();

    let mut h = Harness::new(ExcelApp::new(Some(path.clone())));
    h.key(Key::Escape);
    h.enter_cell("2");
    h.run_line("exit");
    answer(&mut h, dialogs::Choice::Save);
    assert!(
        matches!(&h.app.dialog, Some(Dialog::Confirm { confirmation: excel_command::Confirmation::Loss { .. }, then }) if then == &["exit!"]),
        "{:?}",
        h.app.dialog
    );
    assert!(!h.app.allow_close);
    answer(&mut h, dialogs::Choice::Proceed);
    assert!(h.app.allow_close && !h.app.doc.dirty);
}

#[test]
fn tab_completes_a_file_path_to_open() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("매출 2026.xlsx");
    let mut writer = rust_xlsxwriter::Workbook::new();
    writer.add_worksheet().write_number(0, 0, 42.0).unwrap();
    writer.save(&file).unwrap();

    let mut h = blank();
    h.key_with(Key::L, Modifiers::COMMAND);
    h.typed(&format!("open {}/매", dir.path().display()));
    assert_eq!(h.app.command_line.offered(), ["매출 2026.xlsx"]);
    h.key(Key::Tab);
    assert_eq!(h.app.command_line.text(), format!("open \"{}\"", file.display()));
    h.key(Key::Enter);
    assert_eq!(h.value("A1"), CellValue::Number(42.0));
}

#[test]
fn saving_a_bare_name_adds_xlsx() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = blank();
    h.enter_cell("1");
    h.run_line(&format!("save {}/report", dir.path().display()));
    assert!(dir.path().join("report.xlsx").exists());
    assert!(!h.app.doc.dirty);
}

#[test]
fn ctrl_s_without_a_file_name_asks_for_one() {
    let mut h = blank();
    h.enter_cell("1");
    h.key_with(Key::S, Modifiers::COMMAND);
    assert_eq!(h.app.focus, Focus::CommandLine);
    assert_eq!(h.app.command_line.text(), "save ");
    assert!(h.app.doc.dirty);
}

#[test]
fn opening_a_formatted_file_warns_until_dismissed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("styled.xlsx");
    let mut writer = rust_xlsxwriter::Workbook::new();
    let bold = rust_xlsxwriter::Format::new().set_bold();
    writer.add_worksheet().write_number_with_format(0, 0, 1.0, &bold).unwrap();
    writer.save(&path).unwrap();

    let mut h = Harness::new(ExcelApp::new(Some(path)));
    assert!(matches!(&h.app.dialog, Some(Dialog::LossWarning(items)) if items == &["cell formatting"]));
    h.typed("x");
    assert!(!h.app.grid.is_editing(), "typing must not reach a cell behind the dialog");
    h.key(Key::Escape);
    assert!(h.app.dialog.is_none());
    h.typed("7");
    h.key(Key::Enter);
    assert_eq!(h.value("A1"), CellValue::Number(7.0));
}

#[test]
fn opening_a_bad_file_starts_blank_with_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.xlsx");
    std::fs::write(&path, "not a zip").unwrap();
    let h = Harness::new(ExcelApp::new(Some(path.clone())));
    assert!(h.app.doc.workbook.active().is_empty());
    assert_eq!(h.app.doc.path, None, "saving must not overwrite the file it could not read");
}
