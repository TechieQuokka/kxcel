//! Fitting a column's width to its contents.

use egui::{Color32, FontId, Ui};
use excel_model::{DEFAULT_COL_WIDTH, Sheet};

use crate::paint::{CELL_FONT_SIZE, CELL_PADDING};

/// Narrowest width a fitted column gets, so it stays easy to grab.
const MIN_FITTED_WIDTH: f32 = 24.0;

/// The width that shows every value in a column in full, as cells draw them.
/// An empty column returns to the default width.
pub(crate) fn fitted_width(ui: &Ui, sheet: &Sheet, col: u32) -> f32 {
    let font = FontId::monospace(CELL_FONT_SIZE);
    let widest = sheet
        .iter()
        .filter(|(addr, _)| addr.col == col)
        .map(|(addr, _)| sheet.shown(addr))
        .filter(|text| !text.is_empty())
        .map(|text| ui.painter().layout_no_wrap(text, font.clone(), Color32::WHITE).size().x)
        .fold(None, |widest: Option<f32>, width| Some(widest.map_or(width, |w| w.max(width))));
    match widest {
        Some(width) => (width + 2.0 * CELL_PADDING + 2.0).ceil().max(MIN_FITTED_WIDTH),
        None => DEFAULT_COL_WIDTH,
    }
}
