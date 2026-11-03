use super::*;

#[test]
fn f1_toggles_help_without_taking_the_keyboard_from_the_grid() {
    let mut h = blank();
    h.key(Key::F1);
    assert!(h.app.help.is_open());
    assert!(h.app.help.search_rect().is_some_and(|rect| rect.left() > WINDOW.x / 2.0), "help sits on the right");
    h.enter_cell("5");
    assert_eq!(h.value("A1"), CellValue::Number(5.0));
    h.key(Key::F1);
    assert!(!h.app.help.is_open());
}

#[test]
fn the_help_command_opens_help_on_a_topic() {
    let mut h = blank();
    h.run_line("help sort");
    assert!(h.app.help.is_open());
    assert_eq!(h.app.help.query(), "sort");
    assert_eq!(h.app.help.page_name(), Some("sort"), "an exact name opens its page");
    assert_eq!(h.app.focus, Focus::CommandLine);
    h.run_line("help so");
    assert_eq!(h.app.help.page_name(), None, "a partial name lists matches");
}

#[test]
fn help_has_tabs_and_search_finds_the_tab_with_matches() {
    let mut h = blank();
    h.key(Key::F1);
    assert_eq!(h.app.help.tab(), 0, "functions first");
    let search = h.app.help.search_rect().unwrap().center();
    click(&mut h, search);
    h.typed("goto");
    assert_eq!(h.app.help.tab(), 1, "only the commands tab has a match");
    h.key(Key::Enter);
    assert_eq!(h.app.help.page_name(), Some("goto"));
    h.key(Key::Escape);
    h.key(Key::Escape);
    assert_eq!(h.app.focus, Focus::Grid);

    h.run_line("help vlookup");
    assert_eq!((h.app.help.tab(), h.app.help.page_name()), (0, Some("VLOOKUP")));
    h.key(Key::F1);
    h.key(Key::F1);
    assert_eq!(h.app.help.tab(), 0, "help opens on the last tab");
}

#[test]
fn help_pages_open_with_the_keyboard_and_escape_goes_back() {
    let mut h = blank();
    h.key(Key::F1);
    let search = h.app.help.search_rect().unwrap().center();
    click(&mut h, search);
    h.typed("goto");
    h.key(Key::Enter);
    assert_eq!(h.app.help.page_name(), Some("goto"), "a single match opens with Enter");
    h.key(Key::Escape);
    assert_eq!(h.app.help.page_name(), None);
    assert_eq!(h.app.focus, Focus::Help, "the first escape only goes back");

    h.key_with(Key::A, Modifiers::COMMAND);
    h.typed("m");
    h.key(Key::ArrowDown);
    h.key(Key::ArrowDown);
    h.key(Key::Enter);
    assert!(h.app.help.page_name().is_some(), "arrows pick among several matches");
    h.key(Key::Escape);
    h.key(Key::Escape);
    assert_eq!(h.app.focus, Focus::Grid);
    assert!(h.app.help.is_open());
}

#[test]
fn help_search_takes_typing_and_escape_returns_to_the_grid() {
    let mut h = blank();
    h.key(Key::F1);
    let search = h.app.help.search_rect().unwrap().center();
    click(&mut h, search);
    assert_eq!(h.app.focus, Focus::Help);
    h.typed("sum");
    assert_eq!(h.app.help.query(), "sum");
    assert!(!h.app.grid.is_editing(), "typing in search must not reach a cell");
    h.key(Key::Escape);
    assert_eq!(h.app.focus, Focus::Grid);
    assert!(h.app.help.is_open(), "escape leaves the search but keeps help open");
    h.enter_cell("1");
    assert_eq!(h.value("A1"), CellValue::Number(1.0));
}

#[test]
fn clicking_the_grid_leaves_help_search() {
    let mut h = blank();
    h.key(Key::F1);
    let search = h.app.help.search_rect().unwrap().center();
    click(&mut h, search);
    click(&mut h, cell_center(2, 1));
    assert_eq!(h.app.focus, Focus::Grid);
    assert_eq!(h.app.grid.cursor.to_string(), "B3");
}
