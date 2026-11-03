use excel_model::{CellAddr, CellRange, MAX_COLS, MAX_ROWS, RowSet};

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
    /// How many more functions match than the list shows.
    pub(crate) hints_more: usize,
    pub(crate) hint_picked: usize,
    /// The text the current hints were offered for.
    pub(crate) hints_for: String,
    /// True after Escape closed the list, until the text changes.
    pub(crate) hints_dismissed: bool,
    /// Tab pressed this frame, with whether Shift was held. Taken before the
    /// text box sees it, and acted on after.
    pub(crate) pending_tab: Option<bool>,
    /// The sort and filter popup of a table header, when open.
    pub(crate) popup: Option<crate::FilterPopup>,
    /// App mode: no headers, only input cells can be edited, and `Tab` and
    /// `Enter` move between input cells. Set by the application.
    pub app_mode: bool,
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
            hints_more: 0,
            hint_picked: 0,
            hints_for: String::new(),
            hints_dismissed: false,
            pending_tab: None,
            popup: None,
            app_mode: false,
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

    /// How many more functions match than the list shows.
    pub fn more_functions(&self) -> usize {
        self.hints_more
    }

    /// The open sort and filter popup, if any.
    pub fn filter_popup(&self) -> Option<&crate::FilterPopup> {
        self.popup.as_ref()
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

    /// Resets everything, as when a different document opens. App mode stays.
    pub fn reset(&mut self) {
        *self = Self { app_mode: self.app_mode, ..Self::default() };
    }

    /// The row header's width and the column header's height.
    pub(crate) fn header_size(&self) -> egui::Vec2 {
        if self.app_mode { egui::Vec2::ZERO } else { egui::vec2(ROW_HEADER_WIDTH, COL_HEADER_HEIGHT) }
    }

    /// Moves for `Enter` or `Tab`: by `rows` and `cols`, or in app mode, to
    /// the next input cell, or the one before when going up or left.
    pub(crate) fn step(&mut self, rows: i64, cols: i64, sheet: &excel_model::Sheet) {
        let inputs = &sheet.meta().input_cells;
        if self.app_mode
            && let Some(next) = inputs.next_after(self.cursor, rows > 0 || cols > 0)
        {
            self.jump_to(next, false);
            return;
        }
        self.move_cursor(rows, cols, false, sheet.hidden_rows());
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.arrows_commit = false;
        self.text.clear();
    }

    /// Moves the cursor by an offset, counting only rows that are not hidden.
    /// With `extend`, the selection anchor stays put.
    pub(crate) fn move_cursor(&mut self, rows: i64, cols: i64, extend: bool, hidden: &RowSet) {
        let row = step_rows(self.cursor.row, rows, hidden);
        let col = (i64::from(self.cursor.col) + cols).clamp(0, i64::from(MAX_COLS) - 1);
        self.cursor = CellAddr::new(row, col as u32);
        if !extend {
            self.anchor = self.cursor;
        }
        self.reveal = true;
    }

    /// Moves the cursor and anchor off hidden rows, as after a filter hides
    /// the row they were on.
    pub(crate) fn leave_hidden_rows(&mut self, hidden: &RowSet) {
        let shown = |row| hidden.skip(row, true).or_else(|| hidden.skip(row, false)).unwrap_or(row);
        self.cursor.row = shown(self.cursor.row);
        self.anchor.row = shown(self.anchor.row);
    }

    /// Moves the cursor to `addr`. With `extend`, the selection anchor stays put.
    pub(crate) fn jump_to(&mut self, addr: CellAddr, extend: bool) {
        self.cursor = clamp(addr);
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

    pub(crate) fn scroll_by(&mut self, rows: i64, cols: i64, hidden: &RowSet) {
        self.top_row = step_rows(self.top_row, rows, hidden);
        self.left_col = (i64::from(self.left_col) + cols).clamp(0, i64::from(MAX_COLS) - 1) as u32;
    }

    /// Scrolls the smallest distance that shows the cursor.
    ///
    /// Only rows near the cursor are measured, so jumping far down a large
    /// sheet costs no more than a screen of rows. Hidden rows are passed over.
    pub(crate) fn ensure_cursor_visible(
        &mut self,
        col_width: impl Fn(u32) -> f32,
        row_height: impl Fn(u32) -> f32,
        hidden: &RowSet,
        grid_width: f32,
        grid_height: f32,
    ) {
        let header = self.header_size();
        let available = (grid_height - header.y).max(1.0);
        let shown_above = |row: u32| row.checked_sub(1).and_then(|row| hidden.skip(row, false));
        if self.cursor.row < self.top_row {
            self.top_row = self.cursor.row;
        } else {
            // Walk up from the cursor while rows fit. Reaching the top row means
            // everything from it to the cursor already shows.
            let mut top = self.cursor.row;
            let mut used = row_height(top);
            let mut fits = true;
            while let Some(above) = shown_above(top).filter(|above| *above >= self.top_row) {
                let height = row_height(above);
                if used + height > available {
                    fits = false;
                    break;
                }
                top = above;
                used += height;
            }
            if !fits || used > available {
                self.top_row = top;
            }
        }

        if self.cursor.col < self.left_col {
            self.left_col = self.cursor.col;
        } else {
            let available = (grid_width - header.x).max(1.0);
            let width_from = |left: u32| (left..=self.cursor.col).map(&col_width).sum::<f32>();
            while self.left_col < self.cursor.col && width_from(self.left_col) > available {
                self.left_col += 1;
            }
        }
    }
}

/// The row `count` shown rows away from `row`, stopping at the sheet's edges.
/// Short moves count shown rows one at a time. Long ones, such as a far
/// jump, land by position and then step off any hidden run.
fn step_rows(row: u32, count: i64, hidden: &RowSet) -> u32 {
    let down = count > 0;
    if count.unsigned_abs() > 1000 || hidden.is_empty() {
        let target = (i64::from(row) + count).clamp(0, i64::from(MAX_ROWS) - 1) as u32;
        return hidden.skip(target, down).or_else(|| hidden.skip(target, !down)).unwrap_or(row);
    }
    let mut row = row;
    for _ in 0..count.unsigned_abs() {
        let next = if down { row.checked_add(1).filter(|r| *r < MAX_ROWS) } else { row.checked_sub(1) };
        match next.and_then(|next| hidden.skip(next, down)) {
            Some(next) => row = next,
            None => break,
        }
    }
    row
}

fn clamp(addr: CellAddr) -> CellAddr {
    CellAddr::new(addr.row.min(MAX_ROWS - 1), addr.col.min(MAX_COLS - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    static NONE: RowSet = RowSet::new();

    fn state_with_view() -> GridState {
        let mut state = GridState::default();
        state.set_visible(11, 5);
        state
    }

    #[test]
    fn moving_collapses_the_selection_unless_extending() {
        let mut state = state_with_view();
        state.move_cursor(2, 1, true, &NONE);
        assert_eq!(state.selection().to_string(), "A1:B3");
        state.move_cursor(1, 0, false, &NONE);
        assert_eq!(state.selection().to_string(), "B4");
    }

    #[test]
    fn cursor_stops_at_sheet_edges() {
        let mut state = state_with_view();
        state.move_cursor(-5, -5, false, &NONE);
        assert_eq!(state.cursor, CellAddr::new(0, 0));
        state.move_cursor(i64::from(MAX_ROWS) + 5, 0, false, &NONE);
        assert_eq!(state.cursor.row, MAX_ROWS - 1);
    }

    #[test]
    fn scrolls_down_to_show_the_cursor() {
        let mut state = state_with_view();
        state.move_cursor(20, 0, false, &NONE);
        state.ensure_cursor_visible(|_| 100.0, |_| 24.0, &NONE, 544.0, 264.0);
        assert_eq!(state.top_row, 11);
    }

    #[test]
    fn scrolls_up_to_show_the_cursor() {
        let mut state = state_with_view();
        state.top_row = 50;
        state.move_cursor(3, 0, false, &NONE);
        state.ensure_cursor_visible(|_| 100.0, |_| 24.0, &NONE, 544.0, 264.0);
        assert_eq!(state.top_row, 3);
    }

    #[test]
    fn scrolls_right_to_show_the_cursor() {
        let mut state = state_with_view();
        state.move_cursor(0, 9, false, &NONE);
        state.ensure_cursor_visible(|_| 100.0, |_| 24.0, &NONE, 544.0, 264.0);
        assert_eq!(state.left_col, 6);
    }

    #[test]
    fn a_far_jump_measures_only_rows_near_the_target() {
        let mut state = state_with_view();
        let measured = std::cell::Cell::new(0);
        state.move_cursor(999_999, 0, false, &NONE);
        state.ensure_cursor_visible(
            |_| 100.0,
            |_| {
                measured.set(measured.get() + 1);
                24.0
            },
            &NONE,
            544.0,
            264.0,
        );
        assert_eq!(state.top_row, 999_990);
        assert!(measured.get() < 50, "measured {} rows", measured.get());
    }

    #[test]
    fn tall_rows_scroll_further() {
        let mut state = state_with_view();
        state.move_cursor(3, 0, false, &NONE);
        state.ensure_cursor_visible(|_| 100.0, |row| if row < 3 { 100.0 } else { 24.0 }, &NONE, 544.0, 264.0);
        assert_eq!(state.top_row, 1, "rows 0 to 3 do not fit in 240 points, but rows 1 to 3 do");
    }

    #[test]
    fn moving_and_scrolling_pass_over_hidden_rows() {
        let mut state = state_with_view();
        let hidden: RowSet = [1, 2, 3, 7].into_iter().collect();
        state.move_cursor(1, 0, false, &hidden);
        assert_eq!(state.cursor.row, 4);
        state.move_cursor(3, 0, true, &hidden);
        assert_eq!(state.cursor.row, 8, "rows 5, 6, then 8");
        state.move_cursor(-5, 0, false, &hidden);
        assert_eq!(state.cursor.row, 0);
        state.scroll_by(2, 0, &hidden);
        assert_eq!(state.top_row, 5);
        let mut far = RowSet::default();
        far.insert_run(10, 900_000);
        state.cursor.row = 5;
        state.move_cursor(800_000, 0, false, &far);
        assert_eq!(state.cursor.row, 900_001, "a far jump into a hidden run lands past it");
        state.cursor.row = 20;
        state.leave_hidden_rows(&far);
        assert_eq!(state.cursor.row, 900_001);
    }

    #[test]
    fn showing_the_cursor_counts_only_shown_rows() {
        let mut state = state_with_view();
        let mut hidden = RowSet::default();
        hidden.insert_run(3, 500_000);
        state.cursor.row = 500_005;
        let measured = std::cell::Cell::new(0);
        let height = |_| {
            measured.set(measured.get() + 1);
            24.0
        };
        state.ensure_cursor_visible(|_| 100.0, height, &hidden, 544.0, 264.0);
        assert_eq!(state.top_row, 0, "rows 0 to 2 and 500,001 to 500,005 fit");
        assert!(measured.get() < 20);
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
