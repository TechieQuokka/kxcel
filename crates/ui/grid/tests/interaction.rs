//! Drives the grid through simulated window input, without opening a window.

use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};
use excel_model::{Cell, CellAddr, CellValue, Sheet};
use excel_ui_grid::{FunctionInfo, GridAction, GridState, show};

// With a 56px row header, a 24px column header, and 96px columns.
const fn cell_center(row: u32, col: u32) -> Pos2 {
    pos2(56.0 + 96.0 * col as f32 + 48.0, 24.0 + 24.0 * row as f32 + 12.0)
}

struct Harness {
    ctx: egui::Context,
    state: GridState,
    sheet: Sheet,
    actions: Vec<GridAction>,
    functions: Vec<FunctionInfo>,
    time: f64,
}

impl Harness {
    fn new() -> Self {
        let functions = ["AVERAGE", "MAX", "MIN", "SUM"]
            .map(|name| FunctionInfo { name: name.to_owned(), help: String::new() })
            .to_vec();
        Self::with_functions(functions)
    }

    fn with_functions(functions: Vec<FunctionInfo>) -> Self {
        let mut harness = Self {
            ctx: egui::Context::default(),
            state: GridState::default(),
            sheet: Sheet::new("S"),
            actions: Vec::new(),
            functions,
            time: 0.0,
        };
        harness.frame(Vec::new());
        harness.frame(Vec::new());
        harness
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.frame_with(events, Modifiers::NONE);
    }

    fn frame_with(&mut self, mut events: Vec<Event>, modifiers: Modifiers) {
        events.insert(0, Event::ModifiersChanged(modifiers));
        self.time += 0.1;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let Self { state, sheet, actions, functions, .. } = self;
        let mut output = self.ctx.run_ui(input, |ui| {
            actions.extend(show(ui, state, sheet, true, functions));
        });
        output.textures_delta.clear();
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        let press = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers };
        self.frame_with(vec![press(true), press(false)], modifiers);
    }

    fn typed(&mut self, text: &str) {
        self.frame(vec![Event::Text(text.to_owned())]);
    }

    fn press_at(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![pointer(pos, true)]);
    }

    fn drag_to(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos)]);
    }

    fn release_at(&mut self, pos: Pos2) {
        self.frame(vec![pointer(pos, false)]);
        self.frame(Vec::new());
    }

    fn set_number(&mut self, addr: &str, n: f64) {
        self.sheet.set(addr.parse().unwrap(), Some(Cell::literal(CellValue::Number(n))));
    }
}

fn pointer(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn commit(addr: &str, text: &str) -> GridAction {
    GridAction::Commit { addr: addr.parse().unwrap(), text: text.to_owned() }
}

fn addr(text: &str) -> CellAddr {
    text.parse().unwrap()
}

#[test]
fn arrows_move_and_shift_extends() {
    let mut h = Harness::new();
    h.key(Key::ArrowDown, Modifiers::NONE);
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.state.cursor, addr("B2"));
    h.key(Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(h.state.selection().to_string(), "B2:B3");
}

#[test]
fn typing_starts_an_edit_and_enter_commits_then_moves_down() {
    let mut h = Harness::new();
    h.typed("4");
    h.typed("2");
    assert!(h.state.is_editing());
    assert_eq!(h.state.edit_text(), Some("42"));
    h.key(Key::Enter, Modifiers::NONE);
    assert_eq!(h.actions, [commit("A1", "42")]);
    assert_eq!(h.state.cursor, addr("A2"));
    assert!(!h.state.is_editing());
}

#[test]
fn tab_commits_and_moves_right() {
    let mut h = Harness::new();
    h.typed("x");
    h.key(Key::Tab, Modifiers::NONE);
    assert_eq!(h.actions, [commit("A1", "x")]);
    assert_eq!(h.state.cursor, addr("B1"));
}

#[test]
fn arrow_commits_a_typed_edit() {
    let mut h = Harness::new();
    h.typed("x");
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.actions, [commit("A1", "x")]);
    assert_eq!(h.state.cursor, addr("B1"));
}

#[test]
fn escape_cancels_an_edit() {
    let mut h = Harness::new();
    h.typed("x");
    h.key(Key::Escape, Modifiers::NONE);
    assert!(!h.state.is_editing());
    assert!(h.actions.is_empty());
}

#[test]
fn f2_edits_existing_content_and_arrows_stay_in_the_text() {
    let mut h = Harness::new();
    h.set_number("A1", 5.0);
    h.key(Key::F2, Modifiers::NONE);
    assert_eq!(h.state.edit_text(), Some("5"));
    h.key(Key::ArrowLeft, Modifiers::NONE);
    assert!(h.state.is_editing());
    assert_eq!(h.state.cursor, addr("A1"));
    h.typed("1");
    assert_eq!(h.state.edit_text(), Some("15"));
}

#[test]
fn delete_clears_the_selection() {
    let mut h = Harness::new();
    h.key(Key::ArrowDown, Modifiers::SHIFT);
    h.key(Key::Delete, Modifiers::NONE);
    assert_eq!(h.actions, [GridAction::Clear("A1:A2".parse().unwrap())]);
}

#[test]
fn composing_korean_starts_an_edit_from_the_first_key() {
    let mut h = Harness::new();
    h.frame(vec![preedit("ㅎ")]);
    assert!(h.state.is_editing(), "composition should start an edit");
    h.frame(vec![preedit("하")]);
    h.frame(vec![preedit("한")]);
    h.frame(vec![Event::Ime(egui::ImeEvent::Commit("한".into()))]);
    assert_eq!(h.state.edit_text(), Some("한"));
}

#[test]
fn a_commit_and_enter_in_one_frame_stay_in_the_same_cell() {
    let mut h = Harness::new();
    h.frame(vec![preedit("글")]);
    h.frame(vec![
        Event::Ime(egui::ImeEvent::Commit("글".into())),
        Event::Key { key: Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
    ]);
    h.frame(Vec::new());
    assert_eq!(h.actions, [commit("A1", "글")]);
    assert_eq!(h.state.cursor, addr("A2"));
    assert!(!h.state.is_editing());
    assert_eq!(h.state.edit_text(), None);
}

fn preedit(text: &str) -> Event {
    Event::Ime(egui::ImeEvent::Preedit { text: text.to_owned(), active_range_chars: None })
}

#[test]
fn edited_text_lines_up_with_cell_text() {
    let mut h = Harness::new();
    h.key(Key::ArrowDown, Modifiers::NONE);
    h.key(Key::ArrowRight, Modifiers::NONE);
    h.typed("x");
    h.frame(Vec::new());
    let text = h.state.editor_text_rect().unwrap();
    let cell = cell_center(1, 1);
    assert!((text.center().y - cell.y).abs() <= 1.0, "text at y {}, cell center at {}", text.center().y, cell.y);
    assert!((text.left() - (cell.x - 48.0 + 6.0)).abs() <= 1.0, "text at x {}", text.left());
}

#[test]
fn the_list_waits_for_two_letters() {
    let mut h = Harness::new();
    h.typed("=");
    assert!(h.state.offered_functions().is_empty(), "nothing for = alone");
    h.typed("m");
    assert!(h.state.offered_functions().is_empty(), "nothing for one letter, such as a cell address");
    h.typed("a");
    assert_eq!(h.state.offered_functions(), ["MAX"]);
}

#[test]
fn the_list_shows_at_most_eight() {
    let functions: Vec<FunctionInfo> =
        (0..11).map(|n| FunctionInfo { name: format!("SUM{}", (b'A' + n) as char), help: String::new() }).collect();
    let mut h = Harness::with_functions(functions);
    h.typed("=su");
    assert_eq!(h.state.offered_functions().len(), 8);
    assert_eq!(h.state.more_functions(), 3);
    h.typed("mk");
    assert_eq!(h.state.offered_functions(), ["SUMK"]);
    assert_eq!(h.state.more_functions(), 0);
}

#[test]
fn typing_narrows_the_list_and_tab_completes() {
    let mut h = Harness::new();
    h.typed("=");
    h.typed("ma");
    assert_eq!(h.state.offered_functions(), ["MAX"]);
    h.key(Key::Tab, Modifiers::NONE);
    h.frame(Vec::new());
    assert_eq!(h.state.edit_text(), Some("=MAX("));
    assert!(h.state.is_editing(), "tab completes rather than finishing the edit");
    assert!(h.state.offered_functions().is_empty());
    h.typed("1");
    assert_eq!(h.state.edit_text(), Some("=MAX(1"), "typing continues at the end");
}

#[test]
fn arrows_pick_from_the_list_instead_of_finishing_the_edit() {
    let functions = ["MIN", "MINA"].map(|name| FunctionInfo { name: name.to_owned(), help: String::new() }).to_vec();
    let mut h = Harness::with_functions(functions);
    h.typed("=mi");
    h.key(Key::ArrowDown, Modifiers::NONE);
    assert!(h.state.is_editing());
    assert_eq!(h.state.cursor, addr("A1"));
    h.key(Key::Tab, Modifiers::NONE);
    h.frame(Vec::new());
    assert_eq!(h.state.edit_text(), Some("=MINA("));
}

#[test]
fn completion_replaces_only_the_name_being_typed() {
    let mut h = Harness::new();
    h.typed("=A1+su");
    h.key(Key::Tab, Modifiers::NONE);
    h.frame(Vec::new());
    assert_eq!(h.state.edit_text(), Some("=A1+SUM("));
}

#[test]
fn escape_closes_the_list_first_and_then_cancels() {
    let mut h = Harness::new();
    h.typed("=su");
    h.key(Key::Escape, Modifiers::NONE);
    assert!(h.state.is_editing());
    assert!(h.state.offered_functions().is_empty());
    h.typed("m");
    assert_eq!(h.state.offered_functions(), ["SUM"], "typing reopens the list");
    h.key(Key::Escape, Modifiers::NONE);
    h.key(Key::Escape, Modifiers::NONE);
    assert!(!h.state.is_editing());
}

#[test]
fn clicking_a_listed_function_completes_it() {
    let mut h = Harness::new();
    h.typed("=su");
    // The list opens just below the cell, with a small title above the first item.
    let item = pos2(56.0 + 40.0, 24.0 + 24.0 + 32.0);
    h.frame(vec![Event::PointerMoved(item)]);
    h.frame(vec![pointer(item, true)]);
    h.frame(vec![pointer(item, false)]);
    h.frame(Vec::new());
    assert_eq!(h.state.edit_text(), Some("=SUM("));
    assert_eq!(h.state.cursor, addr("A1"));
}

#[test]
fn plain_text_and_finished_references_offer_nothing() {
    let mut h = Harness::new();
    h.typed("sum");
    assert!(h.state.offered_functions().is_empty());
    h.key(Key::Enter, Modifiers::NONE);
    assert_eq!(h.actions, [commit("A1", "sum")]);
    h.typed("=A1");
    assert!(h.state.offered_functions().is_empty());
    h.key(Key::Tab, Modifiers::NONE);
    assert_eq!(h.actions.last(), Some(&commit("A2", "=A1")), "without a list, tab finishes the edit");
}

#[test]
fn alt_enter_adds_a_line_and_the_editor_grows() {
    let mut h = Harness::new();
    h.typed("one");
    // The first frame of an edit still has the one-pixel box, so measure after it.
    h.frame(Vec::new());
    let single = h.state.editor_text_rect().unwrap().height();
    h.key(Key::Enter, Modifiers::ALT);
    h.typed("two");
    h.frame(Vec::new());
    assert_eq!(h.state.edit_text(), Some("one\ntwo"));
    let double = h.state.editor_text_rect().unwrap().height();
    assert!(double > single * 1.5, "two lines are taller than one: {single} then {double}");
    h.key(Key::Enter, Modifiers::NONE);
    assert_eq!(h.actions, [commit("A1", "one\ntwo")]);
}

#[test]
fn alt_enter_outside_an_edit_does_nothing() {
    let mut h = Harness::new();
    h.key(Key::Enter, Modifiers::ALT);
    assert!(!h.state.is_editing());
    assert_eq!(h.state.cursor, addr("A1"));
}

#[test]
fn tab_never_types_a_tab_character() {
    let mut h = Harness::new();
    h.typed("x");
    h.key(Key::Tab, Modifiers::NONE);
    assert_eq!(h.actions, [commit("A1", "x")]);
    h.key(Key::Tab, Modifiers::NONE);
    assert_eq!(h.state.cursor, addr("C1"));
    assert!(!h.state.is_editing());
    h.key(Key::Tab, Modifiers::SHIFT);
    assert_eq!(h.state.cursor, addr("B1"));
}

#[test]
fn arrows_move_inside_multi_line_text_instead_of_finishing() {
    let mut h = Harness::new();
    h.typed("a");
    h.key(Key::Enter, Modifiers::ALT);
    h.typed("b");
    h.key(Key::ArrowUp, Modifiers::NONE);
    assert!(h.state.is_editing());
    assert!(h.actions.is_empty());
}

#[test]
fn a_multi_line_cell_makes_its_row_taller() {
    let mut h = Harness::new();
    h.sheet.set(addr("A1"), Some(Cell::literal(CellValue::Text("x\ny\nz".into()))));
    h.frame(Vec::new());
    // Where the second row would be if every row were one line tall.
    let usual_second_row = cell_center(1, 0);
    h.press_at(usual_second_row);
    h.release_at(usual_second_row);
    assert_eq!(h.state.cursor, addr("A1"), "the first row now reaches down here");
}

#[test]
fn undo_and_redo_shortcuts() {
    let mut h = Harness::new();
    h.key(Key::Z, Modifiers::COMMAND);
    h.key(Key::Y, Modifiers::COMMAND);
    assert_eq!(h.actions, [GridAction::Undo, GridAction::Redo]);
}

#[test]
fn ctrl_l_goes_to_the_command_line_and_commits_an_edit() {
    let mut h = Harness::new();
    h.typed("x");
    h.key(Key::L, Modifiers::COMMAND);
    assert_eq!(h.actions, [commit("A1", "x"), GridAction::FocusCommandLine]);
}

#[test]
fn clipboard_events_become_actions_instead_of_text() {
    let mut h = Harness::new();
    h.frame(vec![Event::Copy]);
    h.frame(vec![Event::Cut]);
    h.frame(vec![Event::Paste("1\t2".into())]);
    assert_eq!(h.actions.len(), 3);
    assert!(!h.state.is_editing());
}

#[test]
fn pasting_into_an_edit_stays_in_the_text() {
    let mut h = Harness::new();
    h.typed("a");
    h.frame(vec![Event::Paste("b".into())]);
    assert_eq!(h.state.edit_text(), Some("ab"));
    assert!(h.actions.is_empty());
}

#[test]
fn dragging_selects_a_range() {
    let mut h = Harness::new();
    h.press_at(cell_center(1, 1));
    h.drag_to(cell_center(3, 2));
    h.drag_to(cell_center(3, 2));
    h.release_at(cell_center(3, 2));
    assert_eq!(h.state.selection().to_string(), "B2:C4");
}

#[test]
fn dragging_the_fill_handle_reports_a_fill() {
    let mut h = Harness::new();
    h.press_at(cell_center(0, 0));
    h.release_at(cell_center(0, 0));
    // The handle sits on the corner of A1.
    h.press_at(pos2(56.0 + 94.0, 24.0 + 22.0));
    h.drag_to(cell_center(3, 0));
    h.drag_to(cell_center(3, 0));
    h.release_at(cell_center(3, 0));
    assert_eq!(
        h.actions,
        [GridAction::Fill { source: "A1".parse().unwrap(), target: "A1:A4".parse().unwrap(), alternate: false }]
    );
    assert_eq!(h.state.selection().to_string(), "A1:A4");
}

#[test]
fn dragging_the_fill_handle_back_clears_the_cells_left_out() {
    let mut h = Harness::new();
    h.press_at(cell_center(0, 0));
    h.drag_to(cell_center(4, 0));
    h.drag_to(cell_center(4, 0));
    h.release_at(cell_center(4, 0));
    assert_eq!(h.state.selection().to_string(), "A1:A5");
    // The handle sits on the corner of A5.
    h.press_at(pos2(56.0 + 94.0, 24.0 + 24.0 * 5.0 - 2.0));
    h.drag_to(cell_center(2, 0));
    h.drag_to(cell_center(2, 0));
    h.release_at(cell_center(2, 0));
    assert_eq!(h.actions, [GridAction::Clear("A4:A5".parse().unwrap())]);
    assert_eq!(h.state.selection().to_string(), "A1:A3");
}

#[test]
fn ctrl_on_release_marks_an_alternate_fill() {
    let mut h = Harness::new();
    h.press_at(pos2(56.0 + 94.0, 24.0 + 22.0));
    h.drag_to(cell_center(3, 0));
    h.drag_to(cell_center(3, 0));
    let pos = cell_center(3, 0);
    h.frame_with(vec![pointer(pos, false)], Modifiers::COMMAND);
    assert!(matches!(h.actions.last(), Some(GridAction::Fill { alternate: true, .. })));
}

#[test]
fn dragging_during_formula_entry_inserts_a_range() {
    let mut h = Harness::new();
    h.typed("=SUM(");
    h.press_at(cell_center(1, 1));
    assert_eq!(h.state.edit_text(), Some("=SUM(B2"));
    h.drag_to(cell_center(4, 1));
    h.drag_to(cell_center(4, 1));
    h.release_at(cell_center(4, 1));
    assert_eq!(h.state.edit_text(), Some("=SUM(B2:B5"));
    assert_eq!(h.state.cursor, addr("A1"));
    h.typed(")");
    h.key(Key::Enter, Modifiers::NONE);
    assert_eq!(h.actions, [commit("A1", "=SUM(B2:B5)")]);
}

#[test]
fn clicking_another_cell_commits_a_plain_edit() {
    let mut h = Harness::new();
    h.typed("x");
    h.press_at(cell_center(2, 2));
    h.release_at(cell_center(2, 2));
    assert_eq!(h.actions, [commit("A1", "x")]);
    assert_eq!(h.state.cursor, addr("C3"));
}

#[test]
fn double_click_edits_the_cell() {
    let mut h = Harness::new();
    h.set_number("B2", 7.0);
    let pos = cell_center(1, 1);
    h.frame(vec![Event::PointerMoved(pos)]);
    h.frame(vec![pointer(pos, true)]);
    h.frame(vec![pointer(pos, false)]);
    h.frame(vec![pointer(pos, true)]);
    h.frame(vec![pointer(pos, false)]);
    h.frame(Vec::new());
    assert_eq!(h.state.edit_text(), Some("7"));
    assert_eq!(h.state.cursor, addr("B2"));
}

#[test]
fn dragging_a_column_edge_reports_its_width() {
    let mut h = Harness::new();
    let edge = pos2(56.0 + 96.0, 12.0);
    h.press_at(edge);
    h.drag_to(edge + vec2(40.0, 0.0));
    h.drag_to(edge + vec2(40.0, 0.0));
    h.release_at(edge + vec2(40.0, 0.0));
    assert_eq!(h.actions.last(), Some(&GridAction::ResizeColumn { col: 0, width: 136.0 }));
}

fn double_click(h: &mut Harness, pos: Pos2) {
    h.frame(vec![Event::PointerMoved(pos)]);
    h.frame(vec![pointer(pos, true)]);
    h.frame(vec![pointer(pos, false)]);
    h.frame(vec![pointer(pos, true)]);
    h.frame(vec![pointer(pos, false)]);
    h.frame(Vec::new());
}

fn fitted_width(h: &Harness) -> f32 {
    match h.actions.last() {
        Some(GridAction::ResizeColumn { col: 1, width }) => *width,
        other => panic!("expected a resize of column B, got {other:?}"),
    }
}

/// The edge between the B and C column headers.
const B_EDGE: Pos2 = pos2(56.0 + 96.0 * 2.0, 12.0);

#[test]
fn double_clicking_a_column_edge_fits_long_contents() {
    let mut h = Harness::new();
    h.sheet.set(addr("B1"), Some(Cell::literal(CellValue::Text("a fairly long piece of text".into()))));
    // Tests have no Korean font, so the widest value is in English.
    h.sheet.set(
        addr("B900"),
        Some(Cell::literal(CellValue::Text("the widest value is far below the visible rows".into()))),
    );
    double_click(&mut h, B_EDGE);
    let width = fitted_width(&h);
    assert!(width > 300.0, "fits the widest value even when scrolled out of view: {width}");
    assert!(!h.state.is_editing(), "the header double-click must not start a cell edit");
}

#[test]
fn double_clicking_a_column_edge_shrinks_short_contents() {
    let mut h = Harness::new();
    h.set_number("B1", 7.0);
    double_click(&mut h, B_EDGE);
    let width = fitted_width(&h);
    assert!((24.0..60.0).contains(&width), "{width}");
}

#[test]
fn double_clicking_an_empty_column_edge_restores_the_default_width() {
    let mut h = Harness::new();
    double_click(&mut h, B_EDGE);
    assert_eq!(fitted_width(&h), excel_model::DEFAULT_COL_WIDTH);
}

#[test]
fn keyboard_moves_scroll_far_down_a_large_sheet() {
    let mut h = Harness::new();
    h.state.go_to(addr("A1000000"));
    h.frame(Vec::new());
    assert!(h.state.top_row > 999_000);
    assert!(h.state.top_row <= 999_999);
}

#[test]
fn the_wheel_scrolls_by_whole_rows() {
    let mut h = Harness::new();
    h.frame(vec![Event::PointerMoved(cell_center(2, 2))]);
    for _ in 0..30 {
        h.frame(vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, -48.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: Modifiers::NONE,
        }]);
    }
    assert!(h.state.top_row > 0);
    assert_eq!(h.state.cursor, addr("A1"));
}

#[test]
fn tab_closes_one_open_parenthesis_at_a_time_and_stays() {
    let mut h = Harness::new();
    h.typed("=SUM(A1:A3, MAX(B1");
    h.key(Key::Tab, Modifiers::NONE);
    h.frame(Vec::new());
    assert!(h.state.is_editing(), "the formula waits for more");
    assert_eq!(h.state.edit_text(), Some("=SUM(A1:A3, MAX(B1)"), "only the innermost closes");
    assert_eq!(h.state.cursor, addr("A1"));
    assert!(matches!(h.actions.last(), Some(GridAction::Notice(text)) if text.contains("1 still open")));

    h.typed(", C1");
    h.key(Key::Tab, Modifiers::NONE);
    h.frame(Vec::new());
    assert_eq!(h.state.edit_text(), Some("=SUM(A1:A3, MAX(B1), C1)"), "the outer function took another argument");
    assert!(matches!(h.actions.last(), Some(GridAction::Notice(text)) if text.contains("Press Tab or Enter")));

    h.key(Key::Tab, Modifiers::NONE);
    assert_eq!(h.actions.last(), Some(&commit("A1", "=SUM(A1:A3, MAX(B1), C1)")));
    assert_eq!(h.state.cursor, addr("B1"));
}

#[test]
fn only_tab_closes_parentheses() {
    let mut h = Harness::new();
    h.typed("=SUM(A1");
    h.key(Key::Enter, Modifiers::NONE);
    assert_eq!(h.actions.last(), Some(&commit("A1", "=SUM(A1")), "enter enters it as typed");
    h.typed("=LEFT(\"(\"");
    h.key(Key::Tab, Modifiers::NONE);
    h.frame(Vec::new());
    assert_eq!(h.state.edit_text(), Some("=LEFT(\"(\")"), "a parenthesis inside text does not count");
}

#[test]
fn completion_comes_before_closing_parentheses() {
    let mut h = Harness::new();
    h.typed("=SUM(ma");
    h.key(Key::Tab, Modifiers::NONE);
    h.frame(Vec::new());
    assert_eq!(h.state.edit_text(), Some("=SUM(MAX("));
    h.typed("1");
    h.key(Key::Tab, Modifiers::NONE);
    h.frame(Vec::new());
    assert_eq!(h.state.edit_text(), Some("=SUM(MAX(1)"));
}

#[test]
fn ctrl_arrows_jump_to_the_data_edge() {
    let mut h = Harness::new();
    for row in 1..=4 {
        h.set_number(&format!("A{row}"), 1.0);
    }
    h.set_number("A9", 2.0);
    h.key(Key::ArrowDown, Modifiers::COMMAND);
    assert_eq!(h.state.cursor, addr("A4"));
    h.key(Key::ArrowDown, Modifiers::COMMAND);
    assert_eq!(h.state.cursor, addr("A9"));
    h.key(Key::ArrowDown, Modifiers::COMMAND);
    assert_eq!(h.state.cursor, addr("A1048576"));
    assert!(h.state.top_row > 1_000_000, "the view follows the cursor");
    h.key(Key::ArrowUp, Modifiers::COMMAND);
    assert_eq!(h.state.cursor, addr("A9"));
    h.key(Key::ArrowRight, Modifiers::COMMAND);
    assert_eq!(h.state.cursor, addr("XFD9"));
    assert_eq!(h.state.selection().to_string(), "XFD9", "without Shift the selection collapses");
}

#[test]
fn ctrl_shift_arrows_extend_the_selection() {
    let mut h = Harness::new();
    for row in 1..=5 {
        h.set_number(&format!("B{row}"), 1.0);
    }
    h.key(Key::ArrowRight, Modifiers::NONE);
    h.key(Key::ArrowDown, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(h.state.selection().to_string(), "B1:B5");
}

#[test]
fn ctrl_arrows_leave_an_edit_alone() {
    let mut h = Harness::new();
    h.set_number("A5", 1.0);
    h.key(Key::F2, Modifiers::NONE);
    h.typed("abc");
    h.key(Key::ArrowDown, Modifiers::COMMAND);
    assert!(h.state.is_editing());
    assert_eq!(h.state.cursor, addr("A1"));
}

fn table_sheet(h: &mut Harness) {
    use excel_model::{SheetMeta, Table};
    h.sheet.set(addr("A1"), Cell::parse_input("이름"));
    h.sheet.set(addr("A2"), Cell::parse_input("사과"));
    h.sheet.set(addr("A3"), Cell::parse_input("배"));
    h.sheet.set_meta(SheetMeta { tables: vec![Table::new("T", "A1:A3".parse().unwrap())], ..Default::default() });
}

/// The `▼` at the right of a header cell.
fn header_button(row: u32, col: u32) -> Pos2 {
    cell_center(row, col) + vec2(40.0, 0.0)
}

#[test]
fn clicking_a_header_button_opens_its_popup_and_esc_closes_it() {
    let mut h = Harness::new();
    table_sheet(&mut h);
    h.frame(Vec::new());
    h.press_at(header_button(0, 0));
    h.release_at(header_button(0, 0));
    let popup = h.state.filter_popup().expect("open");
    assert_eq!(popup.name, "이름");
    assert_eq!(popup.values.len(), 2);
    assert_eq!(h.state.cursor, addr("A1"), "the table commands act at the cursor");
    h.typed("x");
    assert!(!h.state.is_editing(), "typing goes to the popup, not the cell");
    h.key(Key::Escape, Modifiers::NONE);
    assert!(h.state.filter_popup().is_none());
}

#[test]
fn clicking_outside_closes_the_popup_and_the_header_text_does_not_open_it() {
    let mut h = Harness::new();
    table_sheet(&mut h);
    h.frame(Vec::new());
    h.press_at(cell_center(0, 0));
    h.release_at(cell_center(0, 0));
    assert!(h.state.filter_popup().is_none(), "only the button opens it");
    h.press_at(header_button(0, 0));
    h.release_at(header_button(0, 0));
    assert!(h.state.filter_popup().is_some());
    h.press_at(cell_center(10, 5));
    h.release_at(cell_center(10, 5));
    assert!(h.state.filter_popup().is_none());
    assert!(h.actions.iter().all(|action| !matches!(action, GridAction::RunCommand(_))), "closing changes nothing");
}

fn app_mode_harness() -> Harness {
    let mut h = Harness::new();
    h.state.app_mode = true;
    let mut meta = h.sheet.meta().clone();
    meta.input_cells.add("B2:C2".parse().unwrap());
    meta.input_cells.add("B4".parse().unwrap());
    h.sheet.set_meta(meta);
    h.frame(Vec::new());
    h
}

#[test]
fn app_mode_refuses_edits_outside_input_cells() {
    let mut h = app_mode_harness();
    h.typed("5");
    assert!(!h.state.is_editing());
    assert!(h.actions.iter().any(|a| matches!(a, GridAction::Notice(text) if text.contains("input cells"))));
    h.key(Key::F2, Modifiers::NONE);
    assert!(!h.state.is_editing(), "F2 is refused too");
    h.state.go_to(addr("B2"));
    h.frame(Vec::new());
    h.typed("5");
    assert!(h.state.is_editing());
    h.key(Key::Enter, Modifiers::NONE);
    assert!(h.actions.contains(&commit("B2", "5")));
}

#[test]
fn app_mode_tab_and_enter_go_through_input_cells_in_order() {
    let mut h = app_mode_harness();
    h.key(Key::Tab, Modifiers::NONE);
    assert_eq!(h.state.cursor, addr("B2"));
    h.key(Key::Tab, Modifiers::NONE);
    assert_eq!(h.state.cursor, addr("C2"));
    h.key(Key::Enter, Modifiers::NONE);
    assert_eq!(h.state.cursor, addr("B4"));
    h.key(Key::Tab, Modifiers::NONE);
    assert_eq!(h.state.cursor, addr("B2"), "wraps around");
    h.key(Key::Tab, Modifiers::SHIFT);
    assert_eq!(h.state.cursor, addr("B4"));
    h.typed("7");
    h.key(Key::Tab, Modifiers::NONE);
    assert!(h.actions.contains(&commit("B4", "7")));
    assert_eq!(h.state.cursor, addr("B2"), "finishing an edit with Tab goes on to the next input");
    h.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(h.state.cursor, addr("B3"), "arrows still reach any cell");
}

#[test]
fn app_mode_has_no_headers() {
    let mut h = app_mode_harness();
    // Without headers, the first cell starts at the top left corner.
    h.press_at(pos2(48.0, 12.0));
    h.release_at(pos2(48.0, 12.0));
    assert_eq!(h.state.cursor, addr("A1"));
    h.press_at(pos2(96.0 + 48.0, 24.0 + 12.0));
    h.release_at(pos2(96.0 + 48.0, 24.0 + 12.0));
    assert_eq!(h.state.cursor, addr("B2"));
}
