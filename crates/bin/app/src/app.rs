use std::path::PathBuf;

use egui::{CentralPanel, Frame, Panel, Ui, ViewportCommand};
use excel_command::{Calculator, CommandError, CommandRegistry, Context, Effect, Theme};
use excel_edit::Document;
use excel_features::{Services, registry};
use excel_ui_command_line::{CommandLineAction, CommandLineState};
use excel_ui_grid::{FunctionInfo, GridAction, GridState};
use excel_ui_help_panel::{HelpAction, HelpSection, HelpState};
use excel_ui_status_bar::{SummaryCache, SummaryKey};

use crate::actions::{self, CopiedCells};
use crate::args::{AppMode, Launch};
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
    /// Set when the workbook runs as an app: only its grid, only input cells
    /// changeable, and no commands.
    app_mode: Option<AppMode>,
    /// Where the cursor was last frame. In app mode, a message stays in the
    /// status bar until the cursor moves.
    last_cursor: excel_model::CellAddr,
    /// The cell the latest grid action changed by finishing an edit, so a
    /// table row added after it joins its undo step.
    committed: Option<excel_model::CellAddr>,
    /// The status bar's sum and table rows, worked out again only when the
    /// document, sheet, selection, or cursor changed.
    summary: SummaryCache,
}

impl ExcelApp {
    #[cfg(test)]
    pub fn new(path: Option<PathBuf>) -> Self {
        Self::launch(Launch { path, app: None })
    }

    /// Opens what the command line asked for, as a workbook or as an app.
    pub fn launch(launch: Launch) -> Self {
        let mut app = Self::open(launch.path);
        app.grid.app_mode = launch.app.is_some();
        app.app_mode = launch.app;
        app
    }

    fn open(path: Option<PathBuf>) -> Self {
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
            app_mode: None,
            last_cursor: Default::default(),
            committed: None,
            summary: SummaryCache::default(),
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
        let action = match self.app_mode {
            Some(_) => match self.app_action(action) {
                Some(action) => action,
                None => return,
            },
            None => action,
        };
        let calculator = &self.services.calculator;
        match action {
            GridAction::Commit { addr, text } => {
                let changed = actions::commit(&mut self.doc, calculator, addr, &text);
                self.committed = changed.then_some(addr);
            }
            GridAction::AddTableRow(addr) => {
                // Only right after the edit it follows, so they undo together.
                let index = self.doc.workbook.active_index();
                if self.committed.take() == Some(addr)
                    && let Some(edit) = excel_edit::add_table_row(self.doc.workbook.active(), index, addr)
                {
                    self.doc.apply_joined(edit);
                }
            }
            GridAction::Clear(range) => actions::clear(&mut self.doc, calculator, range),
            GridAction::Fill { source, target, alternate } => {
                actions::fill(&mut self.doc, calculator, source, target, alternate)
            }
            GridAction::Copy(range) => self.capture(ctx, range, false),
            GridAction::Cut(range) => self.capture(ctx, range, true),
            GridAction::Paste { at, text } => actions::paste(&mut self.doc, calculator, &mut self.copied, at, &text),
            GridAction::PasteValues { at, text } => actions::paste(&mut self.doc, calculator, &mut None, at, &text),
            GridAction::RequestPaste => ctx.send_viewport_cmd(ViewportCommand::RequestPaste),
            GridAction::ResizeColumn { col, width } => {
                self.doc.workbook.active_mut().set_col_width(col, width);
                self.doc.touch();
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
            GridAction::RunCommand(line) => {
                self.run_lines(ctx, vec![line]);
            }
        }
    }

    /// Copies or cuts a range. With hidden rows inside it, only the rows that
    /// show are copied, as values, so pasting puts them next to each other and
    /// a cut leaves its cells in place.
    /// What app mode lets through of a grid action. Only input cells change:
    /// a clear or paste reaches only them, a cut copies, and fills, commands,
    /// and the command line are off.
    fn app_action(&mut self, action: GridAction) -> Option<GridAction> {
        let sheet = self.doc.workbook.active();
        match action {
            GridAction::Commit { addr, .. } if !sheet.is_input(addr) => None,
            GridAction::Clear(range) => {
                let calculator = &self.services.calculator;
                let inputs: Vec<_> = sheet.meta().input_cells.overlapping(range).collect();
                actions::clear_within(&mut self.doc, calculator, range, &inputs);
                None
            }
            GridAction::Paste { at, text } | GridAction::PasteValues { at, text } => {
                let inputs = sheet.meta().input_cells.clone();
                let calculator = &self.services.calculator;
                actions::paste_into(&mut self.doc, calculator, &mut self.copied, at, &text, |addr| {
                    inputs.contains(addr)
                });
                None
            }
            GridAction::Cut(range) => Some(GridAction::Copy(range)),
            GridAction::Fill { .. }
            | GridAction::RunCommand(_)
            | GridAction::FocusCommandLine
            | GridAction::AddTableRow(_) => None,
            action => Some(action),
        }
    }

    fn capture(&mut self, ctx: &egui::Context, range: excel_model::CellRange, cut: bool) {
        let (copied, text) = CopiedCells::capture(&self.doc, range, cut);
        let hidden = self.doc.workbook.active().hidden_rows().count_within(range.start.row, range.end.row) > 0;
        self.copied = (!hidden).then_some(copied);
        ctx.copy_text(text);
    }

    /// Stops a close with unsaved changes and asks first, as `exit` does.
    fn intercept_close(&mut self, ctx: &egui::Context) {
        let requested = ctx.input(|i| i.viewport().close_requested());
        let nosave = self.app_mode.as_ref().is_some_and(|mode| mode.nosave);
        if requested && self.doc.dirty && !self.allow_close && !nosave {
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
        let title = match &self.app_mode {
            // An app is named for itself, and one that never saves has nothing to mark.
            Some(mode) => {
                let name = mode.name.clone().unwrap_or(name);
                if mode.nosave { name } else { format!("{name}{mark}") }
            }
            None => format!("{name}{mark} - kxcel"),
        };
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

    /// In app mode: the latest message, or else the current cell as it shows,
    /// never its formula.
    fn app_content(&self) -> String {
        if let Some(message) = self.command_line.message() {
            return message.to_owned();
        }
        if let Some(text) = self.grid.edit_text() {
            return text.replace('\n', " \u{21b5} ");
        }
        self.doc.workbook.active().shown(self.grid.cursor).replace('\n', " \u{21b5} ")
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
        let app_mode = self.app_mode.clone();
        if !has_dialog && keymap::save_pressed(&ctx) {
            match &app_mode {
                Some(mode) if mode.nosave => self.command_line.set_message("This app does not save"),
                _ => self.save_shortcut(&ctx),
            }
        }
        // The window manager can also leave full screen, so follow what it reports.
        if let Some(fullscreen) = ctx.input(|i| i.viewport().fullscreen) {
            self.fullscreen = fullscreen;
        }
        if !has_dialog && keymap::fullscreen_pressed(&ctx) {
            let notice = self.set_fullscreen(&ctx, None);
            self.command_line.set_message(notice);
        }
        if !has_dialog && keymap::help_pressed(&ctx) && app_mode.is_none() {
            self.help.toggle();
            if !self.help.is_open() && self.focus == Focus::Help {
                self.focus = Focus::Grid;
            }
        }

        let command_focused = self.focus == Focus::CommandLine && !has_dialog;
        let grid_focused = self.focus == Focus::Grid && !has_dialog;

        let surface = Frame::NONE.fill(ctx.global_style().visuals.panel_fill);

        // An app has no command line, and its status bar shows messages instead.
        let command_action = match app_mode {
            Some(_) => None,
            None => {
                Panel::bottom("excel_command_panel")
                    .exact_size(COMMAND_LINE_HEIGHT)
                    .frame(Frame::NONE.fill(ctx.global_style().visuals.extreme_bg_color))
                    .show(ui, |ui| {
                        excel_ui_command_line::show(ui, &mut self.command_line, &self.registry, command_focused)
                    })
                    .inner
            }
        };

        if app_mode.is_some() && self.grid.cursor != self.last_cursor {
            self.command_line.clear_message();
        }
        self.last_cursor = self.grid.cursor;
        let content = if app_mode.is_some() { self.app_content() } else { self.current_content() };
        let label = self.file_label();
        Panel::bottom("excel_status_panel")
            .exact_size(STATUS_BAR_HEIGHT)
            .frame(surface.inner_margin(egui::Margin::symmetric(10, 0)))
            .show(ui, |ui| {
                let key = SummaryKey {
                    workbook: self.doc.workbook.id(),
                    revision: self.doc.revision(),
                    sheet: self.doc.workbook.active_index(),
                    selection: self.grid.selection(),
                    cursor: self.grid.cursor,
                };
                let summary = self.summary.get(key, self.doc.workbook.active());
                excel_ui_status_bar::show(ui, summary, self.grid.selection(), &content, &label);
            });

        let help_focused = self.focus == Focus::Help && !has_dialog;
        let help_action = match app_mode {
            Some(_) => None,
            None => excel_ui_help_panel::show(ui, &mut self.help, &self.help_sections, &help::TABS, help_focused),
        };
        match help_action {
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
mod tests;
