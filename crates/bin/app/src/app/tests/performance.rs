use super::*;

#[test]
fn the_status_bar_adds_up_a_selection_once() {
    let mut h = blank();
    h.enter_cell("2");
    h.enter_cell("3");
    h.drag(cell_center(0, 0), cell_center(4, 0));
    let before = h.app.summary.computed();
    for _ in 0..10 {
        h.frame(Vec::new());
    }
    assert_eq!(h.app.summary.computed(), before, "idle frames reuse the sum");
    h.key(Key::Escape);
    h.run_line("set A3 5");
    h.frame(Vec::new());
    assert!(h.app.summary.computed() > before, "an edit works it out again");
    h.key_with(Key::Z, Modifiers::COMMAND);
    let after_undo = h.app.summary.computed();
    h.frame(Vec::new());
    assert_eq!(h.app.summary.computed(), after_undo);
}

#[test]
fn frames_stay_fast_on_a_large_sheet() {
    let mut doc = Document::default();
    for row in 0..1_000_000u32 {
        doc.workbook.active_mut().set(excel_model::CellAddr::new(row, row % 20), excel_model::Cell::parse_input("1"));
    }
    let mut h = Harness::new(ExcelApp::new(None));
    h.app.doc = doc;
    h.frame(Vec::new());
    let started = std::time::Instant::now();
    for frame in 0..60 {
        h.app.grid.top_row = frame * 1000;
        h.frame(Vec::new());
    }
    let per_frame = started.elapsed() / 60;
    println!("1M cells: {per_frame:?} per frame");
    assert!(per_frame.as_millis() < 100, "took {per_frame:?} per frame");
}

#[test]
fn frames_stay_fast_on_a_large_filtered_sheet() {
    let mut doc = Document::default();
    let sheet = doc.workbook.active_mut();
    for row in 0..300_000u32 {
        sheet.set(excel_model::CellAddr::new(row, 0), excel_model::Cell::parse_input("1"));
    }
    let mut meta = sheet.meta().clone();
    for start in (0..300_000).step_by(1000) {
        meta.hidden_rows.insert_run(start + 1, start + 999);
    }
    sheet.set_meta(meta);
    let mut h = Harness::new(ExcelApp::new(None));
    h.app.doc = doc;
    h.frame(Vec::new());
    let started = std::time::Instant::now();
    for frame in 0..30 {
        h.app.grid.top_row = frame * 7919;
        h.frame(Vec::new());
        h.key(Key::PageDown);
    }
    let per_frame = started.elapsed() / 120;
    assert!(per_frame.as_millis() < 100, "took {per_frame:?} per frame");
}
