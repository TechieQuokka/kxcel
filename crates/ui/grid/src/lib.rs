//! Cell grid widget.
//!
//! The grid never changes the document. It reports what the user did as
//! [`GridAction`]s, and the application applies them.
//!
//! Colors come from `egui::Visuals`, so the application's theme controls them.

mod autofit;
mod clipboard;
mod editor;
mod fill;
mod function_hint;
mod headers;
mod layout;
mod paint;
mod pointer;
mod selection;
mod state;

use egui::{FontId, Rect, Sense, Ui, vec2};
use excel_model::{CellAddr, CellRange, Sheet};

pub use clipboard::{parse_tsv, quote_field, to_tsv};
pub use fill::{fill_target, shrunk_away};
pub use function_hint::{FunctionInfo, function_prefix};
pub use selection::accepts_reference;
pub use state::GridState;

use layout::{Layout, SCROLLBAR_SIZE};

/// Something the user did in the grid.
#[derive(Debug, Clone, PartialEq)]
pub enum GridAction {
    /// Finish editing a cell with this text.
    Commit {
        addr: CellAddr,
        text: String,
    },
    /// Clear every cell in a range.
    Clear(CellRange),
    /// Continue `source` into the larger `target`, which contains it.
    Fill {
        source: CellRange,
        target: CellRange,
        /// Ctrl was held on release: a single number counts up instead of
        /// copying, and a series copies instead of continuing.
        alternate: bool,
    },
    Copy(CellRange),
    Cut(CellRange),
    /// Paste clipboard text with its top-left corner at `at`.
    Paste {
        at: CellAddr,
        text: String,
    },
    ResizeColumn {
        col: u32,
        width: f32,
    },
    Undo,
    Redo,
    /// The user asked to move to the command line.
    FocusCommandLine,
    /// The user clicked the grid while the command line had focus.
    FocusGrid,
}

/// Draws the grid in all available space and handles its input.
///
/// `focused` says whether the grid, rather than the command line, has keyboard
/// focus. `functions` are offered for completion during formula entry.
pub fn show(
    ui: &mut Ui,
    state: &mut GridState,
    sheet: &Sheet,
    focused: bool,
    functions: &[FunctionInfo],
) -> Vec<GridAction> {
    let mut actions = Vec::new();
    let rect = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(rect, Sense::click_and_drag());
    let body = Rect::from_min_size(rect.min, rect.size() - vec2(SCROLLBAR_SIZE, SCROLLBAR_SIZE));

    if focused {
        editor::before_text_input(ui, state, &mut actions);
    }

    let line_height = ui.painter().fonts_mut(|fonts| fonts.row_height(&FontId::monospace(paint::CELL_FONT_SIZE)));
    let row_height = |row| layout::row_height(sheet, row, line_height);
    let col_width = |col| sheet.col_width(col);

    let layout = Layout::new(body, state.top_row, state.left_col, col_width, row_height);
    state.set_visible(layout.visible_rows(), layout.visible_cols());

    pointer::handle(ui, &response, state, sheet, &layout, rect, focused, &mut actions);

    let layout = Layout::new(body, state.top_row, state.left_col, col_width, row_height);
    state.set_visible(layout.visible_rows(), layout.visible_cols());

    paint::cells(ui, state, sheet, &layout);
    headers::paint(ui, state, &layout);
    paint::scrollbars(ui, state, sheet, rect);

    if focused {
        let moved = editor::show(ui, state, &layout, line_height);
        state.refresh_hints(functions);
        let anchor = layout.cell_rect(state.cursor).map_or(layout.body().min, |cell| cell.left_bottom());
        if let Some(index) = function_hint::show(ui, state, anchor) {
            state.hint_picked = index;
            state.accept_hint();
        }
        editor::after_text_input(ui, state, sheet, &mut actions);
        if moved || state.take_reveal() {
            state.ensure_cursor_visible(col_width, row_height, body.width(), body.height());
        }
    } else if state.take_reveal() {
        state.ensure_cursor_visible(col_width, row_height, body.width(), body.height());
    }

    actions
}
