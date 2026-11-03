use egui::{Event, ImeEvent, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};
use excel_model::CellValue;

use super::*;

mod app_mode;
mod cells;
mod command_line;
mod files;
mod help;
mod performance;
mod tables;

const WINDOW: egui::Vec2 = vec2(1000.0, 700.0);

struct Harness {
    ctx: egui::Context,
    app: ExcelApp,
    time: f64,
}

impl Harness {
    fn new(app: ExcelApp) -> Self {
        let mut harness = Self { ctx: egui::Context::default(), app, time: 0.0 };
        crate::theme::apply(&harness.ctx, harness.app.theme);
        harness.frame(Vec::new());
        harness.frame(Vec::new());
        harness
    }

    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.time += 0.1;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), WINDOW)),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let app = &mut self.app;
        let mut output = self.ctx.run_ui(input, |ui| app.show(ui));
        output.textures_delta.clear();
        output
    }

    fn key_with(&mut self, key: Key, modifiers: Modifiers) {
        let event = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers };
        self.frame(vec![Event::ModifiersChanged(modifiers), event(true), event(false)]);
        self.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
        self.frame(Vec::new());
    }

    fn key(&mut self, key: Key) {
        self.key_with(key, Modifiers::NONE);
    }

    fn typed(&mut self, text: &str) {
        self.frame(vec![Event::Text(text.to_owned())]);
        self.frame(Vec::new());
    }

    fn enter_cell(&mut self, text: &str) {
        self.typed(text);
        self.key(Key::Enter);
    }

    fn run_line(&mut self, line: &str) {
        if self.app.focus == Focus::CommandLine {
            // Leaving discards a line put back after a failure.
            self.key(Key::Escape);
        }
        self.key_with(Key::L, Modifiers::COMMAND);
        self.typed(line);
        self.key(Key::Enter);
    }

    fn value(&self, addr: &str) -> CellValue {
        self.app.doc.workbook.active().value(addr.parse().unwrap()).clone()
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![button(from, true)]);
        self.frame(vec![Event::PointerMoved(to)]);
        self.frame(vec![Event::PointerMoved(to)]);
        self.frame(vec![button(to, false)]);
        self.frame(Vec::new());
    }
}

fn blank() -> Harness {
    Harness::new(ExcelApp::new(None))
}

fn cell_center(row: u32, col: u32) -> Pos2 {
    pos2(56.0 + 96.0 * col as f32 + 48.0, 24.0 + 24.0 * row as f32 + 12.0)
}

fn click(h: &mut Harness, pos: Pos2) {
    let button =
        |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    h.frame(vec![Event::PointerMoved(pos)]);
    h.frame(vec![button(true)]);
    h.frame(vec![button(false)]);
    h.frame(Vec::new());
}

/// Answers the open dialog as if a button was clicked.
fn answer(h: &mut Harness, choice: dialogs::Choice) {
    let dialog = h.app.dialog.clone().expect("a dialog is open");
    let lines = dialogs::lines_for(&dialog, choice);
    h.app.dialog = None;
    let ctx = h.ctx.clone();
    h.app.run_lines(&ctx, lines);
    h.frame(Vec::new());
}

/// Asks to close the window, as its close button does. Returns the
/// commands the app sent back.
fn close_window(h: &mut Harness) -> Vec<ViewportCommand> {
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
    output.viewport_output[&egui::ViewportId::ROOT].commands.clone()
}

/// A saved form with an input cell at B2, opened as an app.
fn app_harness(dir: &std::path::Path, nosave: bool) -> (Harness, PathBuf) {
    let path = dir.join("form.xlsx");
    let mut doc = Document::default();
    let sheet = doc.workbook.active_mut();
    sheet.set("A2".parse().unwrap(), excel_model::Cell::parse_input("수량"));
    sheet.set("C2".parse().unwrap(), excel_model::Cell::parse_input("=B2*1000"));
    let mut meta = sheet.meta().clone();
    meta.input_cells.add("B2".parse().unwrap());
    sheet.set_meta(meta);
    excel_command::WorkbookStore::save(&excel_features::FileStore, &doc.workbook, &path).unwrap();
    let mode = AppMode { nosave, name: Some("견적".into()), id: None };
    let h = Harness::new(ExcelApp::launch(Launch { path: Some(path.clone()), app: Some(mode) }));
    (h, path)
}
