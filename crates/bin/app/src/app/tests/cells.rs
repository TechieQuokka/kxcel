use super::*;

#[test]
fn typing_in_cells_calculates() {
    let mut h = blank();
    h.enter_cell("3");
    h.enter_cell("4");
    h.enter_cell("=SUM(A1:A2)");
    assert_eq!(h.value("A3"), CellValue::Number(7.0));
    assert_eq!(h.app.grid.cursor.to_string(), "A4");
    assert!(h.app.doc.dirty);
}

#[test]
fn korean_works_in_cells_and_the_command_line() {
    let mut h = blank();
    let preedit = |text: &str| Event::Ime(ImeEvent::Preedit { text: text.to_owned(), active_range_chars: None });
    h.frame(vec![preedit("ㅅ")]);
    h.frame(vec![preedit("사")]);
    h.frame(vec![Event::Ime(ImeEvent::Commit("사".into()))]);
    h.frame(vec![preedit("과")]);
    h.frame(vec![Event::Ime(ImeEvent::Commit("과".into()))]);
    h.frame(Vec::new());
    h.key(Key::Enter);
    assert_eq!(h.value("A1"), CellValue::Text("사과".into()));

    h.key_with(Key::L, Modifiers::COMMAND);
    h.typed("find ");
    h.frame(vec![preedit("사")]);
    h.frame(vec![Event::Ime(ImeEvent::Commit("사".into()))]);
    h.frame(Vec::new());
    h.key(Key::Enter);
    assert_eq!(h.app.grid.cursor.to_string(), "A1");
}

/// Whether a frame told the system to cancel the composition in progress.
fn interrupts_composition(output: &egui::FullOutput) -> bool {
    output.platform_output.ime.as_ref().is_some_and(|ime| ime.should_interrupt_composition)
}

#[test]
fn composition_is_never_interrupted_while_typing() {
    let preedit = |text: &str| Event::Ime(ImeEvent::Preedit { text: text.to_owned(), active_range_chars: None });
    let mut h = blank();
    for events in [vec![preedit("ㅎ")], vec![preedit("하")], vec![preedit("한")], Vec::new(), Vec::new()] {
        assert!(!interrupts_composition(&h.frame(events)), "in a cell");
    }
    h.key(Key::Escape);

    h.key_with(Key::L, Modifiers::COMMAND);
    h.frame(Vec::new());
    for events in [vec![preedit("ㅎ")], vec![preedit("하")], vec![preedit("한")], Vec::new(), Vec::new()] {
        assert!(!interrupts_composition(&h.frame(events)), "in the command line");
    }
}

#[test]
fn every_built_in_function_can_be_completed_and_calculated() {
    let mut h = blank();
    h.enter_cell("2");
    h.enter_cell("4");
    h.typed("=");
    assert!(h.app.grid.offered_functions().is_empty(), "= alone offers nothing");
    h.typed("sum");
    assert_eq!(h.app.grid.offered_functions(), ["SUM", "SUMIF", "SUMIFS"]);
    h.key(Key::Tab);
    h.typed("A1:A2)");
    h.key(Key::Enter);
    assert_eq!(h.value("A3"), CellValue::Number(6.0));
}

#[test]
fn tab_closes_parentheses_and_says_so() {
    let mut h = blank();
    h.typed("=SUM(1, 2");
    h.key(Key::Tab);
    assert!(h.app.grid.is_editing());
    assert!(h.app.command_line.message().is_some_and(|text| text.contains("Closed the parenthesis")));
    h.key(Key::Enter);
    assert_eq!(h.value("A1"), CellValue::Number(3.0));
}

#[test]
fn every_function_is_offered_once_its_name_is_typed() {
    let names: Vec<&str> = Services::default().calculator.functions().into_iter().map(|(name, _)| name).collect();
    let infos: Vec<FunctionInfo> =
        names.iter().map(|name| FunctionInfo { name: (*name).to_owned(), help: String::new() }).collect();
    for name in names {
        let matching = excel_ui_grid::offered_for(&infos, &format!("={name}"));
        assert!(matching.contains(&name), "{name} is not offered: {matching:?}");
        assert!(matching.len() <= 8);
    }
}

#[test]
fn alt_enter_writes_several_lines_into_one_cell() {
    let mut h = blank();
    h.typed("first");
    h.key_with(Key::Enter, Modifiers::ALT);
    assert!(h.app.grid.is_editing(), "alt+enter must not finish the edit");
    h.typed("둘째");
    h.key(Key::Enter);
    assert_eq!(h.value("A1"), CellValue::Text("first\n둘째".into()));
    assert_eq!(h.app.grid.cursor.to_string(), "A2");
    assert_eq!(h.app.current_content(), "", "A2 is empty");
    h.key(Key::ArrowUp);
    assert_eq!(h.app.current_content(), "first \u{21b5} 둘째");
}

#[test]
fn undo_and_redo_recalculate() {
    let mut h = blank();
    h.enter_cell("2");
    h.enter_cell("=A1*2");
    h.key(Key::ArrowUp);
    h.key(Key::ArrowUp);
    h.enter_cell("5");
    assert_eq!(h.value("A2"), CellValue::Number(10.0));
    h.key_with(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.value("A2"), CellValue::Number(4.0));
    h.key_with(Key::Y, Modifiers::COMMAND);
    assert_eq!(h.value("A2"), CellValue::Number(10.0));
}

#[test]
fn the_fill_handle_copies_a_formula_with_adjusted_references() {
    let mut h = blank();
    h.enter_cell("1");
    h.enter_cell("2");
    h.enter_cell("3");
    h.run_line("set B1 =A1*10");
    h.run_line("goto B1");
    h.drag(pos2(56.0 + 96.0 * 2.0 - 2.0, 24.0 + 22.0), cell_center(2, 1));
    assert_eq!(h.value("B3"), CellValue::Number(30.0));
    assert_eq!(h.app.grid.selection().to_string(), "B1:B3");
}

#[test]
fn a_dragged_range_shows_in_a_formula() {
    let mut h = blank();
    h.enter_cell("1");
    h.enter_cell("2");
    h.typed("=SUM(");
    h.drag(cell_center(0, 0), cell_center(1, 0));
    h.typed(")");
    h.key(Key::Enter);
    assert_eq!(h.value("A3"), CellValue::Number(3.0));
}

#[test]
fn copy_and_paste_go_through_the_clipboard() {
    let mut h = blank();
    h.enter_cell("2");
    h.run_line("set B1 =A1*2");
    h.run_line("goto B1");
    let copied_text = {
        let output = h.frame(vec![Event::Copy]);
        output
            .platform_output
            .commands
            .iter()
            .find_map(|command| match command {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(copied_text, "4");
    h.key(Key::ArrowDown);
    h.frame(vec![Event::Paste(copied_text)]);
    assert_eq!(h.app.doc.workbook.active().get("B2".parse().unwrap()).unwrap().input_text(), "=A2*2");
}

#[test]
fn delete_clears_and_a_failed_command_shows_an_error() {
    let mut h = blank();
    h.enter_cell("1");
    h.key(Key::ArrowUp);
    h.key(Key::Delete);
    assert_eq!(h.value("A1"), CellValue::Empty);
    h.run_line("nope");
    assert_eq!(h.app.focus, Focus::CommandLine);
    let mut ctx = Context::new(&mut h.app.doc, &h.app.services.calculator, &h.app.services.store);
    assert!(h.app.registry.execute("nope", &mut ctx).is_err());
}

#[test]
fn paste_values_leaves_formulas_behind() {
    let mut h = blank();
    h.enter_cell("1234");
    h.run_line("set B1 =A1*2");
    h.run_line("goto B1");
    let output = h.frame(vec![Event::Copy]);
    let copied = output.platform_output.commands.iter().find_map(|command| match command {
        egui::OutputCommand::CopyText(text) => Some(text.clone()),
        _ => None,
    });
    assert_eq!(copied.as_deref(), Some("2,468"), "copies what shows, commas included");
    let ctx = h.ctx.clone();
    let at = "B2".parse().unwrap();
    h.app.apply_grid_action(&ctx, GridAction::PasteValues { at, text: copied.unwrap() });
    let cell = h.app.doc.workbook.active().get(at).unwrap().clone();
    assert_eq!(cell.input_text(), "2468", "a number, not the formula");
    h.app.apply_grid_action(&ctx, GridAction::RequestPaste);
}
