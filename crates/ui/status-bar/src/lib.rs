//! Status bar widget: current cell, its raw content, and selection statistics.

mod selection_stats;

use egui::{Align, Layout, RichText, Ui};
use excel_model::{CellAddr, CellRange, Sheet, format_number_grouped};

pub use selection_stats::{SelectionStats, selection_stats, table_rows};

/// Draws the status bar.
///
/// `content` is the raw text of the current cell, and `file` is what to show
/// on the right, such as the sheet and whether there are unsaved changes. A
/// filtered table at `cursor` also shows how many of its rows show.
pub fn show(ui: &mut Ui, sheet: &Sheet, cursor: CellAddr, selection: CellRange, content: &str, file: &str) {
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        let accent = ui.visuals().selection.stroke.color;
        ui.label(RichText::new(selection.to_string()).monospace().color(accent));

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(file).monospace().weak());
            if let Some(rows) = table_rows(sheet, cursor) {
                ui.label(RichText::new(rows).monospace().color(accent));
            }
            if let Some(stats) = selection_stats(sheet, selection) {
                let text = format!(
                    "Sum {}  Avg {}  Count {}",
                    format_number_grouped(stats.sum),
                    format_number_grouped(stats.average),
                    stats.count
                );
                ui.label(RichText::new(text).monospace());
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add(egui::Label::new(RichText::new(content).monospace()).truncate());
            });
        });
    });
}
