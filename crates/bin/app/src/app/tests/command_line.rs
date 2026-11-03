use super::*;

#[test]
fn a_command_that_moves_the_cursor_returns_to_the_grid() {
    let mut h = blank();
    h.run_line("goto C5");
    assert_eq!(h.app.grid.cursor.to_string(), "C5");
    assert_eq!(h.app.focus, Focus::Grid);
    assert!(!h.app.grid.is_editing(), "the command must not land in a cell");
    h.enter_cell("9");
    assert_eq!(h.value("C5"), CellValue::Number(9.0));
}

#[test]
fn commands_need_no_colon() {
    let mut h = blank();
    h.run_line(":goto C5");
    assert_eq!(h.app.grid.cursor.to_string(), "A1");
}

#[test]
fn other_commands_stay_in_the_command_line() {
    let mut h = blank();
    h.run_line("set A1 5");
    assert_eq!(h.app.focus, Focus::CommandLine);
    assert_eq!(h.app.command_line.text(), "");
    h.typed("set A2 =A1*2");
    h.key(Key::Enter);
    assert_eq!(h.value("A2"), CellValue::Number(10.0));
    assert!(!h.app.grid.is_editing());
}

#[test]
fn a_failed_command_stays_with_its_line_to_fix() {
    let mut h = blank();
    h.run_line("goto ZZZ");
    assert_eq!(h.app.focus, Focus::CommandLine);
    assert_eq!(h.app.command_line.text(), "goto ZZZ");
    for _ in 0..3 {
        h.key(Key::Backspace);
    }
    h.typed("C3");
    h.key(Key::Enter);
    assert_eq!(h.app.grid.cursor.to_string(), "C3");
    assert_eq!(h.app.focus, Focus::Grid);
}

#[test]
fn tab_completes_a_command() {
    let mut h = blank();
    h.key_with(Key::L, Modifiers::COMMAND);
    h.typed("go");
    h.key(Key::Tab);
    assert_eq!(h.app.command_line.text(), "goto ");
    assert_eq!(h.app.focus, Focus::CommandLine);
    h.typed("B2");
    h.key(Key::Enter);
    assert_eq!(h.app.grid.cursor.to_string(), "B2");
}

#[test]
fn escape_leaves_the_command_line_and_discards_its_text() {
    let mut h = blank();
    h.key_with(Key::L, Modifiers::COMMAND);
    h.typed("sort");
    h.key(Key::Escape);
    assert_eq!(h.app.focus, Focus::Grid);
    assert_eq!(h.app.command_line.text(), "");
    h.enter_cell("1");
    assert_eq!(h.value("A1"), CellValue::Number(1.0));
}

/// Whether the latest frame asked the window for full screen, and which way.
fn fullscreen_request(output: &egui::FullOutput) -> Option<bool> {
    output.viewport_output.get(&egui::ViewportId::ROOT)?.commands.iter().find_map(|command| match command {
        ViewportCommand::Fullscreen(on) => Some(*on),
        _ => None,
    })
}

#[test]
fn f11_and_the_fullscreen_command_switch_full_screen() {
    let mut h = blank();
    let press = |h: &mut Harness| {
        let event = |pressed| Event::Key {
            key: Key::F11,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        h.frame(vec![event(true), event(false)])
    };
    assert_eq!(fullscreen_request(&press(&mut h)), Some(true));
    assert!(h.app.fullscreen);
    assert!(h.app.command_line.message().is_some_and(|text| text.contains("F11")));
    assert_eq!(fullscreen_request(&press(&mut h)), Some(false));

    h.run_line("fullscreen on");
    assert!(h.app.fullscreen);
    h.run_line("fullscreen on");
    assert!(h.app.fullscreen, "on stays on");
    h.run_line("fullscreen");
    assert!(!h.app.fullscreen, "no argument switches");
    assert_eq!(h.app.focus, Focus::CommandLine);
}

#[test]
fn the_theme_command_switches_colors_and_remembers_them() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("kxcel").join("settings.conf");
    let mut h = blank();
    h.app.settings_path = Some(path.clone());
    assert!(h.ctx.global_style().visuals.dark_mode, "dark at first");

    h.run_line("theme light");
    h.frame(Vec::new());
    assert_eq!(h.app.theme(), Theme::Light);
    assert!(!h.ctx.global_style().visuals.dark_mode);
    assert_eq!(h.app.command_line.message(), Some("Light theme"));
    assert_eq!(Settings::load(&path).theme, Theme::Light, "remembered for the next start");
    let function_color = h.app.help_sections[0].color;
    assert_eq!(function_color, Some(theme::function_color(Theme::Light)));

    h.run_line("theme");
    h.frame(Vec::new());
    assert_eq!(h.app.theme(), Theme::Dark, "no argument switches");
    assert!(h.ctx.global_style().visuals.dark_mode);
    assert_eq!(Settings::load(&path).theme, Theme::Dark);

    h.run_line("theme blue");
    assert_eq!(h.app.theme(), Theme::Dark);
    assert_eq!(h.app.command_line.text(), "theme blue", "a mistake stays to be fixed");
}

#[test]
fn deleting_a_sheet_asks_with_a_dialog() {
    let mut h = blank();
    h.run_line("sheet add 매출");
    h.run_line("sheet delete");
    assert!(matches!(
        &h.app.dialog,
        Some(Dialog::Confirm { confirmation: excel_command::Confirmation::Ask { question, .. }, .. })
            if question == "Delete sheet 매출?"
    ));
    h.key(Key::Escape);
    assert_eq!(h.app.doc.workbook.sheets().len(), 2, "escape keeps the sheet");

    h.run_line("sheet delete");
    answer(&mut h, dialogs::Choice::Proceed);
    assert_eq!(h.app.doc.workbook.sheets().len(), 1);
    assert_eq!(h.app.command_line.message(), Some("Deleted sheet 매출"));
    h.key(Key::Escape);
    h.key_with(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.doc.workbook.sheets().len(), 2, "undo brings it back");
}

#[test]
fn clicking_moves_focus_between_the_grid_and_the_command_line() {
    let mut h = blank();
    let click = |h: &mut Harness, pos| {
        let button =
            |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        h.frame(vec![Event::PointerMoved(pos)]);
        h.frame(vec![button(true)]);
        h.frame(vec![button(false)]);
        h.frame(Vec::new());
    };
    click(&mut h, pos2(100.0, WINDOW.y - 10.0));
    assert_eq!(h.app.focus, Focus::CommandLine);
    h.typed("goto D4");
    h.key(Key::Enter);
    assert_eq!(h.app.grid.cursor.to_string(), "D4");

    h.key_with(Key::L, Modifiers::COMMAND);
    click(&mut h, cell_center(1, 1));
    assert_eq!(h.app.focus, Focus::Grid);
    assert_eq!(h.app.grid.cursor.to_string(), "B2");
}
