//! Spreadsheet GUI: a cell grid, a status bar, and a command line.

mod actions;
mod app;
mod dialogs;
mod fonts;
mod help;
mod icon;
mod keymap;
mod series;
mod theme;

use std::path::PathBuf;

use app::ExcelApp;

fn main() -> eframe::Result {
    let path = std::env::args_os().nth(1).map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("kxcel")
            .with_app_id("kxcel")
            .with_inner_size([1100.0, 700.0])
            .with_icon(icon::window_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "kxcel",
        options,
        Box::new(|creation| {
            fonts::install(&creation.egui_ctx);
            theme::apply(&creation.egui_ctx);
            Ok(Box::new(ExcelApp::new(path)))
        }),
    )
}
