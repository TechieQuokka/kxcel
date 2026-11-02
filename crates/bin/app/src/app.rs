use std::path::PathBuf;

use egui::{CentralPanel, Frame, Panel, Ui, ViewportCommand};
use excel_command::{Calculator, CommandError, CommandRegistry, Context, Effect, Theme};
use excel_edit::Document;
use excel_features::{Services, registry};
use excel_ui_command_line::{CommandLineAction, CommandLineState};
use excel_ui_grid::{FunctionInfo, GridAction, GridState};
use excel_ui_help_panel::{HelpAction, HelpSection, HelpState};

use crate::actions::{self, CopiedCells};
use crate::dialogs::{self, Dialog};
use crate::settings::{self, Settings};
use crate::{help, keymap, theme};

const COMMAND_LINE_HEIGHT: f32 = 30.0;
const STATUS_BAR_HEIGHT: f32 = 26.0;

/// How running command lines ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ran {
    /// Every line ran. `moved` says whether one moved the cursor.
    Done { moved: bool },
    /// A line stopped to ask, and a dialog is open.
    Asked,
    /// A line failed, and its error is showing.
    Failed,
}

/// Which area receives the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Grid,
    CommandLine,
    Help,
}

pub struct ExcelApp {
    services: Services,
    registry: CommandRegistry,
    functions: Vec<FunctionInfo>,
    help: HelpState,
    help_sections: Vec<HelpSection>,
    doc: Document,
    grid: GridState,
    command_line: CommandLineState,
    focus: Focus,
    copied: Option<CopiedCells>,
    dialog: Option<Dialog>,
    allow_close: bool,
    fullscreen: bool,
    theme: Theme,
    /// Where settings are kept, or `None` to keep them only for this run.
    settings_path: Option<PathBuf>,
    title: String,
}

impl ExcelApp {
    pub fn new(path: Option<PathBuf>) -> Self {
        let services = Services::default();
        let mut command_line = CommandLineState::default();
        let doc = match path {
            Some(path) => services.open(&path).unwrap_or_else(|error| {
                command_line.set_error(error.to_string());
                Document::default()
            }),
            None => Document::default(),
        };
        let dialog = (!doc.loss_warnings.is_empty()).then(|| Dialog::LossWarning(doc.loss_warnings.clone()));
        let functions: Vec<FunctionInfo> = services
            .calculator
            .functions()
            .into_iter()
            .map(|(name, doc)| FunctionInfo { name: name.to_owned(), help: format!("{}  {}", doc.usage, doc.summary) })
            .collect();
        let registry = registry();
        let settings_path = settings::default_path();
        let theme = settings_path.as_deref().map(Settings::load).unwrap_or_default().theme;
        let help_sections = help::sections(&registry, &services.calculator.functions(), theme);
        Self {
            services,
            registry,
            functions,
            help: HelpState::default(),
            help_sections,
            doc,
            grid: GridState::default(),
            command_line,
            focus: Focus::Grid,
            copied: None,
            dialog,
            allow_close: false,
            fullscreen: false,
            theme,
            settings_path,
            title: String::new(),
        }
    }

    /// Runs command lines in order, stopping at the first that fails or asks to
    /// confirm. A question opens a dialog that carries the remaining lines.
    fn run_lines(&mut self, ctx: &egui::Context, lines: Vec<String>) -> Ran {
        let mut moved = false;
        let mut lines = lines.into_iter();
        while let Some(line) = lines.next() {
            match self.run_command(ctx, &line) {
                Ok(cursor_moved) => moved |= cursor_moved,
                Err(error) => match error.confirmation {
                    Some(confirmation) => {
                        self.command_line.clear_message();
                        self.dialog = Some(Dialog::Confirm { confirmation, then: lines.collect() });
                        return Ran::Asked;
                    }
                    None => {
                        self.command_line.set_error(error.to_string());
                        return Ran::Failed;
                    }
                },
            }
        }
        Ran::Done { moved }
    }

    /// Runs one command line and applies its effects. Returns whether it moved the cursor.
    fn run_command(&mut self, ctx: &egui::Context, line: &str) -> Result<bool, CommandError> {
        let mut command = Context::new(&mut self.doc, &self.services.calculator, &self.services.store);
        command.cursor = self.grid.cursor;
        command.selection = self.grid.selection();
        let result = self.registry.execute(line, &mut command);
        let message = command.message.take();
        let effects = std::mem::take(&mut command.effects);

        let mut moved = false;
        let mut notice = None;
        for effect in effects {
            match effect {
                Effect::MoveCursor(addr) => {
                    self.grid.go_to(addr);
                    moved = true;
                }
                Effect::Quit => {
                    self.allow_close = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                Effect::ShowHelp(topic) => self.help.open(topic),
                Effect::Fullscreen(on) => notice = Some(self.set_fullscreen(ctx, on)),
                Effect::Theme(theme) => notice = Some(self.set_theme(ctx, theme)),
                Effect::DocumentReplaced => {
                    self.grid.reset();
                    self.copied = None;
                    if !self.doc.loss_warnings.is_empty() {
                        self.dialog = Some(Dialog::LossWarning(self.doc.loss_warnings.clone()));
                    }
                }
                _ => {}
            }
        }

        result?;
        match message.or(notice) {
            Some(message) => self.command_line.set_message(message),
            None => self.command_line.clear_message(),
        }
        Ok(moved)
    }

    /// The theme in use.
    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// Uses a theme, or switches to the other for `None`, and remembers it for
    /// the next start. Returns what to tell the user.
    fn set_theme(&mut self, ctx: &egui::Context, choice: Option<Theme>) -> String {
        self.theme = choice.unwrap_or(self.theme.other());
        theme::apply(ctx, self.theme);
        self.help_sections = help::sections(&self.registry, &self.services.calculator.functions(), self.theme);
        let saved = match &self.settings_path {
            Some(path) => Settings { theme: self.theme }.save(path).map_err(|error| error.to_string()),
            None => Ok(()),
        };
        match saved {
            Ok(()) => format!("{} theme", if self.theme == Theme::Dark { "Dark" } else { "Light" }),
            Err(error) => format!("Theme changed, but it could not be saved: {error}"),
        }
    }

    /// Turns full screen on or off, or switches it for `None`. Returns what to tell the user.
    fn set_fullscreen(&mut self, ctx: &egui::Context, on: Option<bool>) -> String {
        self.fullscreen = on.unwrap_or(!self.fullscreen);
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
        if self.fullscreen { "Full screen. F11 or fullscreen off to leave" } else { "Full screen off" }.to_owned()
    }

    /// Runs a line the user typed. A command that moves the cursor returns to
    /// the grid, and anything else stays in the command line. A failed line is
    /// put back so it can be fixed.
    fn submit(&mut self, ctx: &egui::Context, line: String) {
        match self.run_lines(ctx, vec![line.clone()]) {
            Ran::Done { moved } => self.focus = if moved { Focus::Grid } else { Focus::CommandLine },
            Ran::Asked => self.focus = Focus::CommandLine,
            Ran::Failed => {
                self.command_line.set_text(line);
                self.focus = Focus::CommandLine;
            }
        }
    }

    /// Saves from the shortcut. A document without a file name gets a `save `
    /// line to type one into.
    fn save_shortcut(&mut self, ctx: &egui::Context) {
        if self.doc.path.is_none() {
            self.command_line.set_text("save ");
            self.command_line.set_message("Type a file name to save to");
            self.focus = Focus::CommandLine;
            return;
        }
        if self.run_lines(ctx, vec!["save".into()]) == Ran::Failed {
            self.focus = Focus::CommandLine;
        }
    }

    fn apply_grid_action(&mut self, ctx: &egui::Context, action: GridAction) {
        let calculator = &self.services.calculator;
        match action {
            GridAction::Commit { addr, text } => actions::commit(&mut self.doc, calculator, addr, &text),
            GridAction::Clear(range) => actions::clear(&mut self.doc, calculator, range),
            GridAction::Fill { source, target, alternate } => {
                actions::fill(&mut self.doc, calculator, source, target, alternate)
            }
            GridAction::Copy(range) => self.capture(ctx, range, false),
            GridAction::Cut(range) => self.capture(ctx, range, true),
            GridAction::Paste { at, text } => actions::paste(&mut self.doc, calculator, &mut self.copied, at, &text),
            GridAction::ResizeColumn { col, width } => {
                self.doc.workbook.active_mut().set_col_width(col, width);
                self.doc.dirty = true;
            }
            GridAction::Undo => {
                if self.doc.undo()
                    && let Some(edit) = self.doc.history.last_redo()
                {
                    self.services.calculator.update(&mut self.doc.workbook, edit);
                }
            }
            GridAction::Redo => {
                if self.doc.redo()
                    && let Some(edit) = self.doc.history.last_undo()
                {
                    self.services.calculator.update(&mut self.doc.workbook, edit);
                }
            }
            GridAction::FocusCommandLine => self.focus = Focus::CommandLine,
            GridAction::FocusGrid => self.focus = Focus::Grid,
            GridAction::Notice(text) => self.command_line.set_message(text),
        }
    }

    fn capture(&mut self, ctx: &egui::Context, range: excel_model::CellRange, cut: bool) {
        let (copied, text) = CopiedCells::capture(&self.doc, range, cut);
        self.copied = Some(copied);
        ctx.copy_text(text);
    }

    /// Stops a close with unsaved changes and asks first, as `exit` does.
    fn intercept_close(&mut self, ctx: &egui::Context) {
        let requested = ctx.input(|i| i.viewport().close_requested());
        if requested && self.doc.dirty && !self.allow_close {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            if self.dialog.is_none() {
                self.run_lines(ctx, vec!["exit".into()]);
            }
        }
    }

    fn show_dialog(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &self.dialog else {
            return;
        };
        let Some(choice) = dialogs::show(ctx, dialog) else {
            return;
        };
        let lines = dialogs::lines_for(dialog, choice);
        self.dialog = None;
        match self.run_lines(ctx, lines) {
            Ran::Done { moved: true } => self.focus = Focus::Grid,
            Ran::Failed => self.focus = Focus::CommandLine,
            Ran::Done { moved: false } | Ran::Asked => {}
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let name = self
            .doc
            .path
            .as_deref()
            .and_then(|path| path.file_name())
            .map_or_else(|| "untitled".to_owned(), |name| name.to_string_lossy().into_owned());
        let mark = if self.doc.dirty { " *" } else { "" };
        let title = format!("{name}{mark} - kxcel");
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn file_label(&self) -> String {
        let mark = if self.doc.dirty { " *" } else { "" };
        format!("{}{mark}", self.doc.workbook.active().name)
    }

    fn current_content(&self) -> String {
        // The status bar has one line, so line breaks show as a mark.
        self.raw_content().replace('\n', " \u{21b5} ")
    }

    fn raw_content(&self) -> String {
        if let Some(text) = self.grid.edit_text() {
            return text.to_owned();
        }
        let Some(cell) = self.doc.workbook.active().get(self.grid.cursor) else {
            return String::new();
        };
        if cell.uncalculated { format!("{}  (could not be calculated)", cell.input_text()) } else { cell.input_text() }
    }
}

impl eframe::App for ExcelApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

impl ExcelApp {
    /// Runs one frame. Separate from `eframe::App` so tests can run it without a window.
    fn show(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        self.intercept_close(&ctx);
        self.update_title(&ctx);

        let has_dialog = self.dialog.is_some();
        if !has_dialog && keymap::save_pressed(&ctx) {
            self.save_shortcut(&ctx);
        }
        // The window manager can also leave full screen, so follow what it reports.
        if let Some(fullscreen) = ctx.input(|i| i.viewport().fullscreen) {
            self.fullscreen = fullscreen;
        }
        if !has_dialog && keymap::fullscreen_pressed(&ctx) {
            let notice = self.set_fullscreen(&ctx, None);
            self.command_line.set_message(notice);
        }
        if !has_dialog && keymap::help_pressed(&ctx) {
            self.help.toggle();
            if !self.help.is_open() && self.focus == Focus::Help {
                self.focus = Focus::Grid;
            }
        }

        let command_focused = self.focus == Focus::CommandLine && !has_dialog;
        let grid_focused = self.focus == Focus::Grid && !has_dialog;

        let surface = Frame::NONE.fill(ctx.global_style().visuals.panel_fill);

        let command_action = Panel::bottom("excel_command_panel")
            .exact_size(COMMAND_LINE_HEIGHT)
            .frame(Frame::NONE.fill(ctx.global_style().visuals.extreme_bg_color))
            .show(ui, |ui| excel_ui_command_line::show(ui, &mut self.command_line, &self.registry, command_focused))
            .inner;

        let content = self.current_content();
        let label = self.file_label();
        Panel::bottom("excel_status_panel")
            .exact_size(STATUS_BAR_HEIGHT)
            .frame(surface.inner_margin(egui::Margin::symmetric(10, 0)))
            .show(ui, |ui| {
                excel_ui_status_bar::show(ui, self.doc.workbook.active(), self.grid.selection(), &content, &label);
            });

        let help_focused = self.focus == Focus::Help && !has_dialog;
        match excel_ui_help_panel::show(ui, &mut self.help, &self.help_sections, &help::TABS, help_focused) {
            Some(HelpAction::FocusSearch) => self.focus = Focus::Help,
            Some(HelpAction::Leave) => self.focus = Focus::Grid,
            Some(HelpAction::Close) => {
                self.help.close();
                self.focus = Focus::Grid;
            }
            None => {}
        }

        let grid_actions = CentralPanel::no_frame()
            .show(ui, |ui| {
                excel_ui_grid::show(ui, &mut self.grid, self.doc.workbook.active(), grid_focused, &self.functions)
            })
            .inner;

        for action in grid_actions {
            self.apply_grid_action(&ctx, action);
        }

        match command_action {
            Some(CommandLineAction::Submit(line)) => self.submit(&ctx, line),
            Some(CommandLineAction::Leave) => self.focus = Focus::Grid,
            Some(CommandLineAction::Focus) => self.focus = Focus::CommandLine,
            None => {}
        }

        self.show_dialog(&ctx);
    }
}

#[cfg(test)]
mod tests {
    use egui::{Event, ImeEvent, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};
    use excel_model::CellValue;

    use super::*;

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

    fn click(h: &mut Harness, pos: Pos2) {
        let button =
            |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        h.frame(vec![Event::PointerMoved(pos)]);
        h.frame(vec![button(true)]);
        h.frame(vec![button(false)]);
        h.frame(Vec::new());
    }

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

    /// Answers the open dialog as if a button was clicked.
    fn answer(h: &mut Harness, choice: dialogs::Choice) {
        let dialog = h.app.dialog.clone().expect("a dialog is open");
        let lines = dialogs::lines_for(&dialog, choice);
        h.app.dialog = None;
        let ctx = h.ctx.clone();
        h.app.run_lines(&ctx, lines);
        h.frame(Vec::new());
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

    #[test]
    fn clicking_moves_focus_between_the_grid_and_the_command_line() {
        let mut h = blank();
        let click = |h: &mut Harness, pos| {
            let button = |pressed| Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            };
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

    #[test]
    fn frames_stay_fast_on_a_large_sheet() {
        let mut doc = Document::default();
        for row in 0..1_000_000u32 {
            doc.workbook
                .active_mut()
                .set(excel_model::CellAddr::new(row, row % 20), excel_model::Cell::parse_input("1"));
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
}
