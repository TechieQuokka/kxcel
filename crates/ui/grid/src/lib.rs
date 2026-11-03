//! Cell grid widget.
//!
//! The grid never changes the document. It reports what the user did as
//! [`GridAction`]s, and the application applies them.
//!
//! Colors come from `egui::Visuals`, so the application's theme controls them.

mod autofit;
mod clipboard;
mod edge;
mod editor;
mod fill;
mod filter_popup;
mod function_hint;
mod headers;
mod layout;
mod menu;
mod paint;
mod palette;
mod pointer;
mod selection;
mod state;

use egui::{FontId, Rect, Sense, Ui, vec2};
use excel_model::{CellAddr, CellRange, Sheet};

pub use clipboard::{parse_tsv, quote_field, to_tsv};
pub use edge::{Direction, data_edge};
pub use fill::{fill_target, shrunk_away};
pub use filter_popup::FilterPopup;
pub use function_hint::{FunctionInfo, MAX_SHOWN, MIN_TYPED, function_prefix, offered_for};
pub use palette::GridPalette;
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
    /// Something to tell the user, such as parentheses closed by Tab.
    Notice(String),
    /// Run a command line, such as `table create` for `Ctrl+T`. The grid knows
    /// nothing about commands, so it only names the line.
    RunCommand(String),
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

    // While the filter popup is open, its own text boxes take the keyboard.
    if focused && state.popup.is_none() && state.menu.is_none() {
        editor::before_text_input(ui, state, &mut actions);
    }

    let line_height = ui.painter().fonts_mut(|fonts| fonts.row_height(&FontId::monospace(paint::CELL_FONT_SIZE)));
    let row_height = |row| layout::row_height(sheet, row, line_height);
    let col_width = |col| sheet.col_width(col);

    let hidden = sheet.hidden_rows();
    let shown = |row| sheet.shown_row(row, true);
    // A filter may have just hidden the cursor's row.
    state.leave_hidden_rows(hidden);

    let header = state.header_size();
    let layout = Layout::new(body, header, state.top_row, state.left_col, col_width, row_height, shown);
    state.set_visible(layout.visible_rows(), layout.visible_cols());

    pointer::handle(ui, &response, state, sheet, &layout, rect, focused, &mut actions);
    refuse_edits_outside_inputs(state, sheet, &mut actions);

    let layout = Layout::new(body, header, state.top_row, state.left_col, col_width, row_height, shown);
    state.set_visible(layout.visible_rows(), layout.visible_cols());

    paint::cells(ui, state, sheet, &layout);
    if !state.app_mode {
        headers::paint(ui, state, &layout, !hidden.is_empty());
    }
    paint::scrollbars(ui, state, sheet, rect);

    if let Some(popup) = &state.popup {
        let anchor = layout.cell_rect(popup.header).map_or(layout.body().min, |cell| cell.left_bottom());
        filter_popup::show(ui, &mut state.popup, anchor, &mut actions);
    }
    menu::show(ui, state, &mut actions);
    let focused = focused && state.popup.is_none() && state.menu.is_none();

    if focused {
        let moved = editor::show(ui, state, &layout, line_height);
        refuse_edits_outside_inputs(state, sheet, &mut actions);
        state.refresh_hints(functions);
        let anchor = layout.cell_rect(state.cursor).map_or(layout.body().min, |cell| cell.left_bottom());
        if let Some(index) = function_hint::show(ui, state, anchor) {
            state.hint_picked = index;
            state.accept_hint();
        }
        editor::after_text_input(ui, state, sheet, &mut actions);
        refuse_edits_outside_inputs(state, sheet, &mut actions);
        if moved || state.take_reveal() {
            state.ensure_cursor_visible(col_width, row_height, hidden, body.width(), body.height());
        }
    } else if state.take_reveal() {
        state.ensure_cursor_visible(col_width, row_height, hidden, body.width(), body.height());
    }

    actions
}

/// In app mode, an edit may start only in an input cell. Typing, `F2`, or a
/// double-click anywhere else is undone at once, with a note why.
fn refuse_edits_outside_inputs(state: &mut GridState, sheet: &Sheet, actions: &mut Vec<GridAction>) {
    if state.app_mode && state.is_editing() && !sheet.is_input(state.cursor) {
        state.cancel_edit();
        actions.push(GridAction::Notice("Only input cells can be changed".into()));
    }
}
