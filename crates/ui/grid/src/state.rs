use excel_model::{CellAddr, CellRange, MAX_COLS, MAX_ROWS};

use crate::FunctionInfo;
use crate::layout::{COL_HEADER_HEIGHT, ROW_HEADER_WIDTH};

/// A pointer gesture in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Drag {
    Select,
    Fill {
        source: CellRange,
        target: CellRange,
    },
    Resize {
        col: u32,
        start_x: f32,
        start_width: f32,
    },
    /// Choosing a range while typing a formula. `insert_at` is where its text starts.
    FormulaRange {
        start: CellAddr,
        insert_at: usize,
    },
    ScrollVertical,
    ScrollHorizontal,
}

/// The grid's view state: cursor, selection, scroll position, and in-progress edit.
#[derive(Debug, Clone)]
pub struct GridState {
    pub cursor: CellAddr,
    pub anchor: CellAddr,
    pub top_row: u32,
    pub left_col: u32,
    /// The editor's text. Text typed before editing begins also arrives here.
    pub(crate) text: String,
    pub(crate) editing: bool,
    /// True when editing began by typing, so arrow keys finish the edit.
    pub(crate) arrows_commit: bool,
    pub(crate) cursor_to_end: bool,
    pub(crate) drag: Option<Drag>,
    /// The far corner of a range being chosen during formula entry.
    pub(crate) reference_end: CellAddr,
    pub(crate) visible_rows: u32,
    pub(crate) visible_cols: u32,
    pub(crate) scroll_remainder: egui::Vec2,
    /// Where the editor drew its text in the latest frame.
    pub(crate) editor_text_rect: Option<egui::Rect>,
    pub(crate) hints: Vec<FunctionInfo>,
    pub(crate) hint_picked: usize,
    /// The text the current hints were offered for.
    pub(crate) hints_for: String,
    /// True after Escape closed the list, until the text changes.
    pub(crate) hints_dismissed: bool,
    /// Tab pressed this frame, with whether Shift was held. Taken before the
    /// text box sees it, and acted on after.
    pub(crate) pending_tab: Option<bool>,
    reveal: bool,
}

impl Default for GridState {
    fn default() -> Self {
        Self {
            cursor: CellAddr::default(),
            anchor: CellAddr::default(),
            top_row: 0,
            left_col: 0,
            text: String::new(),
            editing: false,
            arrows_commit: false,
            cursor_to_end: false,
            drag: None,
            reference_end: CellAddr::default(),
            visible_rows: 1,
            visible_cols: 1,
            scroll_remainder: egui::Vec2::ZERO,
            editor_text_rect: None,
            hints: Vec::new(),
            hint_picked: 0,
            hints_for: String::new(),
            hints_dismissed: false,
            pending_tab: None,
            reveal: false,
        }
    }
}

impl GridState {
    pub fn selection(&self) -> CellRange {
        CellRange::new(self.anchor, self.cursor)
    }

    /// Where the editor drew its text in the latest frame.
    pub fn editor_text_rect(&self) -> Option<egui::Rect> {
        self.editor_text_rect
    }

    /// The names of the functions currently offered.
    pub fn offered_functions(&self) -> Vec<&str> {
        self.hints.iter().map(|function| function.name.as_str()).collect()
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    /// The text being edited, shown in place of the cell's content elsewhere.
    pub fn edit_text(&self) -> Option<&str> {
        self.editing.then_some(self.text.as_str())
    }

    /// Moves the cursor to `addr`, collapsing the selection, and scrolls it into view.
    pub fn go_to(&mut self, addr: CellAddr) {
        self.cancel_edit();
        self.cursor = clamp(addr);
        self.anchor = self.cursor;
        self.reveal = true;
    }

    /// Resets everything, as when a different document opens.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.arrows_commit = false;
        self.text.clear();
    }

    /// Moves the cursor by an offset. With `extend`, the selection anchor stays put.
    pub(crate) fn move_cursor(&mut self, rows: i64, cols: i64, extend: bool) {
        let row = (i64::from(self.cursor.row) + rows).clamp(0, i64::from(MAX_ROWS) - 1);
        let col = (i64::from(self.cursor.col) + cols).clamp(0, i64::from(MAX_COLS) - 1);
        self.cursor = CellAddr::new(row as u32, col as u32);
        if !extend {
            self.anchor = self.cursor;
        }
        self.reveal = true;
    }

    pub(crate) fn begin_edit(&mut self, text: String) {
        self.text = text;
        self.editing = true;
        self.arrows_commit = false;
        self.cursor_to_end = true;
    }

    pub(crate) fn take_text(&mut self) -> String {
        self.editing = false;
        self.arrows_commit = false;
        std::mem::take(&mut self.text)
    }

    pub(crate) fn take_reveal(&mut self) -> bool {
        std::mem::take(&mut self.reveal)
    }

    pub(crate) fn set_visible(&mut self, rows: u32, cols: u32) {
        self.visible_rows = rows.max(1);
        self.visible_cols = cols.max(1);
    }

    pub(crate) fn scroll_by(&mut self, rows: i64, cols: i64) {
        self.top_row = (i64::from(self.top_row) + rows).clamp(0, i64::from(MAX_ROWS) - 1) as u32;
        self.left_col = (i64::from(self.left_col) + cols).clamp(0, i64::from(MAX_COLS) - 1) as u32;
    }

    /// Scrolls the smallest distance that shows the cursor.
    ///
    /// Only rows near the cursor are measured, so jumping far down a large
    /// sheet costs no more than a screen of rows.
    pub(crate) fn ensure_cursor_visible(
        &mut self,
        col_width: impl Fn(u32) -> f32,
        row_height: impl Fn(u32) -> f32,
        grid_width: f32,
        grid_height: f32,
    ) {
        let available = (grid_height - COL_HEADER_HEIGHT).max(1.0);
        if self.cursor.row < self.top_row {
            self.top_row = self.cursor.row;
        } else {
            let near = self.cursor.row - self.top_row <= self.visible_rows;
            let fits = near && (self.top_row..=self.cursor.row).map(&row_height).sum::<f32>() <= available;
            if !fits {
                // Put the cursor on the last fully visible row.
                let mut top = self.cursor.row;
                let mut used = row_height(top);
                while top > 0 && used + row_height(top - 1) <= available {
                    top -= 1;
                    used += row_height(top);
                }
                self.top_row = top;
            }
        }

        if self.cursor.col < self.left_col {
            self.left_col = self.cursor.col;
        } else {
            let available = (grid_width - ROW_HEADER_WIDTH).max(1.0);
            let width_from = |left: u32| (left..=self.cursor.col).map(&col_width).sum::<f32>();
            while self.left_col < self.cursor.col && width_from(self.left_col) > available {
                self.left_col += 1;
            }
        }
    }
}

fn clamp(addr: CellAddr) -> CellAddr {
    CellAddr::new(addr.row.min(MAX_ROWS - 1), addr.col.min(MAX_COLS - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_view() -> GridState {
        let mut state = GridState::default();
        state.set_visible(11, 5);
        state
    }

    #[test]
    fn moving_collapses_the_selection_unless_extending() {
        let mut state = state_with_view();
        state.move_cursor(2, 1, true);
        assert_eq!(state.selection().to_string(), "A1:B3");
        state.move_cursor(1, 0, false);
        assert_eq!(state.selection().to_string(), "B4");
    }

    #[test]
    fn cursor_stops_at_sheet_edges() {
        let mut state = state_with_view();
        state.move_cursor(-5, -5, false);
        assert_eq!(state.cursor, CellAddr::new(0, 0));
        state.move_cursor(i64::from(MAX_ROWS) + 5, 0, false);
        assert_eq!(state.cursor.row, MAX_ROWS - 1);
    }

    #[test]
    fn scrolls_down_to_show_the_cursor() {
        let mut state = state_with_view();
        state.move_cursor(20, 0, false);
        state.ensure_cursor_visible(|_| 100.0, |_| 24.0, 544.0, 264.0);
        assert_eq!(state.top_row, 11);
    }

    #[test]
    fn scrolls_up_to_show_the_cursor() {
        let mut state = state_with_view();
        state.top_row = 50;
        state.move_cursor(3, 0, false);
        state.ensure_cursor_visible(|_| 100.0, |_| 24.0, 544.0, 264.0);
        assert_eq!(state.top_row, 3);
    }

    #[test]
    fn scrolls_right_to_show_the_cursor() {
        let mut state = state_with_view();
        state.move_cursor(0, 9, false);
        state.ensure_cursor_visible(|_| 100.0, |_| 24.0, 544.0, 264.0);
        assert_eq!(state.left_col, 6);
    }

    #[test]
    fn a_far_jump_measures_only_rows_near_the_target() {
        let mut state = state_with_view();
        let measured = std::cell::Cell::new(0);
        state.move_cursor(999_999, 0, false);
        state.ensure_cursor_visible(
            |_| 100.0,
            |_| {
                measured.set(measured.get() + 1);
                24.0
            },
            544.0,
            264.0,
        );
        assert_eq!(state.top_row, 999_990);
        assert!(measured.get() < 50, "measured {} rows", measured.get());
    }

    #[test]
    fn tall_rows_scroll_further() {
        let mut state = state_with_view();
        state.move_cursor(3, 0, false);
        state.ensure_cursor_visible(|_| 100.0, |row| if row < 3 { 100.0 } else { 24.0 }, 544.0, 264.0);
        assert_eq!(state.top_row, 1, "rows 0 to 3 do not fit in 240 points, but rows 1 to 3 do");
    }

    #[test]
    fn go_to_cancels_editing() {
        let mut state = state_with_view();
        state.begin_edit("abc".into());
        state.go_to(CellAddr::new(4, 1));
        assert!(!state.is_editing());
        assert_eq!(state.selection().to_string(), "B5");
    }
}
