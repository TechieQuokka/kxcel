//! Spreadsheet GUI: a cell grid, a status bar, and a command line.

mod actions;
mod app;
mod args;
mod dialogs;
mod fonts;
mod help;
mod icon;
mod keymap;
mod series;
mod settings;
mod theme;

use app::ExcelApp;

fn main() -> eframe::Result {
    let launch = match args::parse(std::env::args_os().skip(1)) {
        Ok(args::Parsed::Open(launch)) => launch,
        Ok(args::Parsed::Version) => {
            println!("{}", args::VERSION);
            return Ok(());
        }
        Ok(args::Parsed::Help) => {
            println!("{}", args::USAGE);
            return Ok(());
        }
        Err(error) => {
            eprintln!("{error}\n{}", args::USAGE);
            std::process::exit(2);
        }
    };
    let app_id = launch.app.as_ref().and_then(|mode| mode.id.clone()).unwrap_or_else(|| "kxcel".to_owned());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("kxcel")
            .with_app_id(app_id)
            .with_inner_size([1100.0, 700.0])
            .with_icon(icon::window_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "kxcel",
        options,
        Box::new(|creation| {
            fonts::install(&creation.egui_ctx);
            let app = ExcelApp::launch(launch);
            theme::apply(&creation.egui_ctx, app.theme());
            Ok(Box::new(app))
        }),
    )
}
