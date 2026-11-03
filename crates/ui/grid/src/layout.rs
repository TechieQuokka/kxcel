use egui::{Pos2, Rect, Vec2, pos2, vec2};
use excel_model::{CellAddr, CellRange, CellValue, MAX_COLS, MAX_ROWS, Sheet};

/// Height of a row whose cells hold one line.
pub(crate) const ROW_HEIGHT: f32 = 24.0;
pub(crate) const COL_HEADER_HEIGHT: f32 = 24.0;
pub(crate) const ROW_HEADER_WIDTH: f32 = 56.0;
pub(crate) const SCROLLBAR_SIZE: f32 = 12.0;
/// Distance from a column edge within which dragging resizes the column.
pub(crate) const RESIZE_GRAB: f32 = 4.0;
/// Space above and below the text of a multi-line cell.
pub(crate) const LINE_PADDING: f32 = 4.0;

/// A visible column.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ColSpan {
    pub col: u32,
    pub x: f32,
    pub width: f32,
}

/// A visible row.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RowSpan {
    pub row: u32,
    pub y: f32,
    pub height: f32,
}

/// How tall a row must be to show every line of its cells. Rows grow with
/// multi-line text, so heights never need storing.
pub(crate) fn row_height(sheet: &Sheet, row: u32, line_height: f32) -> f32 {
    let whole_row = CellRange { start: CellAddr::new(row, 0), end: CellAddr::new(row, MAX_COLS - 1) };
    let lines = sheet
        .iter_range(whole_row)
        .map(|(_, cell)| match &cell.value {
            CellValue::Text(text) => text.split('\n').count(),
            _ => 1,
        })
        .max()
        .unwrap_or(1);
    if lines <= 1 { ROW_HEIGHT } else { (lines as f32 * line_height + 2.0 * LINE_PADDING).max(ROW_HEIGHT) }
}

/// Screen positions of the visible rows and columns.
///
/// Scrolling moves by whole rows and columns, and only visible cells are ever
/// measured, so positions stay small and precise on a sheet of any size.
#[derive(Debug, Clone)]
pub(crate) struct Layout {
    pub rect: Rect,
    /// The row header's width and the column header's height, zero when
    /// headers are hidden.
    pub header: Vec2,
    pub top_row: u32,
    pub rows: Vec<RowSpan>,
    pub cols: Vec<ColSpan>,
}

impl Layout {
    /// `shown` gives the first row at or after its argument that is not
    /// hidden, or `None` when there is none, so hidden rows take no space.
    pub fn new(
        rect: Rect,
        header: Vec2,
        top_row: u32,
        left_col: u32,
        col_width: impl Fn(u32) -> f32,
        row_height: impl Fn(u32) -> f32,
        shown: impl Fn(u32) -> Option<u32>,
    ) -> Self {
        let mut rows = Vec::new();
        let mut y = rect.top() + header.y;
        let mut next = shown(top_row);
        while let Some(row) = next.filter(|_| y < rect.bottom()) {
            let height = row_height(row);
            rows.push(RowSpan { row, y, height });
            y += height;
            next = row.checked_add(1).filter(|row| *row < MAX_ROWS).and_then(&shown);
        }

        let mut cols = Vec::new();
        let mut x = rect.left() + header.x;
        let mut col = left_col;
        while x < rect.right() && col < MAX_COLS {
            let width = col_width(col);
            cols.push(ColSpan { col, x, width });
            x += width;
            col += 1;
        }
        let top_row = rows.first().map_or(top_row, |span: &RowSpan| span.row);
        Self { rect, header, top_row, rows, cols }
    }

    pub fn visible_rows(&self) -> u32 {
        self.rows.len() as u32
    }

    pub fn visible_cols(&self) -> u32 {
        self.cols.len() as u32
    }

    /// The area holding cells, excluding the headers.
    pub fn body(&self) -> Rect {
        Rect::from_min_max(self.rect.min + self.header, self.rect.max)
    }

    pub fn row_span(&self, row: u32) -> Option<RowSpan> {
        let index = self.rows.binary_search_by_key(&row, |span| span.row).ok()?;
        Some(self.rows[index])
    }

    /// The first shown row at or after `row`, if one is in view.
    pub fn row_span_from(&self, row: u32) -> Option<RowSpan> {
        let index = self.rows.partition_point(|span| span.row < row);
        self.rows.get(index).copied()
    }

    /// The screen rectangle of a cell, or `None` if it is scrolled out of view.
    pub fn cell_rect(&self, addr: CellAddr) -> Option<Rect> {
        let row = self.row_span(addr.row)?;
        let col = self.cols.iter().find(|span| span.col == addr.col)?;
        Some(Rect::from_min_size(pos2(col.x, row.y), vec2(col.width, row.height)))
    }

    /// The cell under a position inside the cell area.
    pub fn cell_at(&self, pos: Pos2) -> Option<CellAddr> {
        if !self.body().contains(pos) {
            return None;
        }
        let row = self.rows.iter().find(|span| pos.y < span.y + span.height)?;
        let col = self.cols.iter().find(|span| pos.x < span.x + span.width)?;
        Some(CellAddr::new(row.row, col.col))
    }

    /// The nearest visible cell. Used while dragging outside the grid.
    pub fn nearest_cell(&self, pos: Pos2) -> Option<CellAddr> {
        let body = self.body();
        let last_col = self.cols.last()?;
        let last_row = self.rows.last()?;
        let clamped = pos2(
            pos.x.clamp(body.left(), (last_col.x + last_col.width - 1.0).min(body.right() - 1.0)),
            pos.y.clamp(body.top(), (last_row.y + last_row.height - 1.0).min(body.bottom() - 1.0)),
        );
        self.cell_at(clamped)
    }

    /// The row whose number is under a position in the row header.
    pub fn row_header_at(&self, pos: Pos2) -> Option<u32> {
        let inside = (self.rect.left()..self.rect.left() + self.header.x).contains(&pos.x);
        inside.then(|| {
            self.rows.iter().find(|span| (span.y..span.y + span.height).contains(&pos.y)).map(|span| span.row)
        })?
    }

    /// The column whose letter is under a position in the column header.
    pub fn col_header_at(&self, pos: Pos2) -> Option<u32> {
        let inside = (self.rect.top()..self.rect.top() + self.header.y).contains(&pos.y);
        inside.then(|| {
            self.cols.iter().find(|span| (span.x..span.x + span.width).contains(&pos.x)).map(|span| span.col)
        })?
    }

    /// The column whose right edge is under a position in the column header.
    pub fn resize_handle_at(&self, pos: Pos2) -> Option<ColSpan> {
        let header_top = self.rect.top()..=self.rect.top() + self.header.y;
        if self.header.y <= 0.0 {
            return None;
        }
        if !header_top.contains(&pos.y) {
            return None;
        }
        self.cols.iter().copied().find(|span| (pos.x - (span.x + span.width)).abs() <= RESIZE_GRAB)
    }
}

#[cfg(test)]
mod tests {
    use excel_model::Cell;

    use super::*;

    const HEADERS: Vec2 = vec2(ROW_HEADER_WIDTH, COL_HEADER_HEIGHT);

    fn layout() -> Layout {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(ROW_HEADER_WIDTH + 300.0, COL_HEADER_HEIGHT + 96.0));
        Layout::new(rect, HEADERS, 10, 2, |_| 100.0, |_| ROW_HEIGHT, Some)
    }

    #[test]
    fn counts_visible_rows_and_columns() {
        let layout = layout();
        assert_eq!(layout.visible_rows(), 4);
        assert_eq!(layout.visible_cols(), 3);
    }

    #[test]
    fn finds_the_cell_under_a_position() {
        let layout = layout();
        let pos = pos2(ROW_HEADER_WIDTH + 150.0, COL_HEADER_HEIGHT + 30.0);
        assert_eq!(layout.cell_at(pos), Some(CellAddr::new(11, 3)));
    }

    #[test]
    fn headers_are_not_cells() {
        assert_eq!(layout().cell_at(pos2(10.0, 50.0)), None);
        assert_eq!(layout().cell_at(pos2(100.0, 10.0)), None);
    }

    #[test]
    fn cell_rect_inverts_cell_at() {
        let layout = layout();
        let addr = CellAddr::new(12, 4);
        assert_eq!(layout.cell_at(layout.cell_rect(addr).unwrap().center()), Some(addr));
    }

    #[test]
    fn scrolled_out_cells_have_no_rect() {
        assert!(layout().cell_rect(CellAddr::new(0, 2)).is_none());
        assert!(layout().cell_rect(CellAddr::new(10, 0)).is_none());
    }

    #[test]
    fn nearest_cell_clamps_to_the_view() {
        let layout = layout();
        assert_eq!(layout.nearest_cell(pos2(-50.0, -50.0)), Some(CellAddr::new(10, 2)));
        assert_eq!(layout.nearest_cell(pos2(9999.0, 9999.0)), Some(CellAddr::new(13, 4)));
    }

    #[test]
    fn finds_resize_handles_in_the_header_only() {
        let layout = layout();
        let edge = ROW_HEADER_WIDTH + 100.0;
        assert_eq!(layout.resize_handle_at(pos2(edge, 10.0)).map(|s| s.col), Some(2));
        assert!(layout.resize_handle_at(pos2(edge, 50.0)).is_none());
    }

    #[test]
    fn stops_at_the_last_row_of_a_sheet() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 500.0));
        let layout = Layout::new(rect, HEADERS, MAX_ROWS - 2, 0, |_| 100.0, |_| ROW_HEIGHT, Some);
        assert_eq!(layout.visible_rows(), 2);
    }

    #[test]
    fn tall_rows_push_later_rows_down() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 500.0));
        let layout = Layout::new(rect, HEADERS, 0, 0, |_| 100.0, |row| if row == 0 { 60.0 } else { ROW_HEIGHT }, Some);
        assert_eq!(layout.row_span(1).unwrap().y, COL_HEADER_HEIGHT + 60.0);
        let inside_first_row = pos2(ROW_HEADER_WIDTH + 10.0, COL_HEADER_HEIGHT + 50.0);
        assert_eq!(layout.cell_at(inside_first_row), Some(CellAddr::new(0, 0)));
    }

    #[test]
    fn hidden_rows_take_no_space() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, COL_HEADER_HEIGHT + 3.0 * ROW_HEIGHT));
        let shown = |row: u32| Some(if (2..=900).contains(&row) { 901 } else { row });
        let layout = Layout::new(rect, HEADERS, 0, 0, |_| 100.0, |_| ROW_HEIGHT, shown);
        assert_eq!(layout.rows.iter().map(|span| span.row).collect::<Vec<_>>(), [0, 1, 901]);
        assert!(layout.row_span(5).is_none());
        assert_eq!(layout.row_span(901).unwrap().y, COL_HEADER_HEIGHT + 2.0 * ROW_HEIGHT);
        assert_eq!(layout.row_span_from(5).unwrap().row, 901);
        let third_row = pos2(ROW_HEADER_WIDTH + 10.0, COL_HEADER_HEIGHT + 2.5 * ROW_HEIGHT);
        assert_eq!(layout.cell_at(third_row), Some(CellAddr::new(901, 0)));
        let from_hidden = Layout::new(rect, HEADERS, 4, 0, |_| 100.0, |_| ROW_HEIGHT, shown);
        assert_eq!(from_hidden.top_row, 901, "a hidden top row starts at the next shown one");
    }

    #[test]
    fn row_height_follows_the_most_lines_in_the_row() {
        let mut sheet = Sheet::new("S");
        sheet.set(CellAddr::new(0, 0), Cell::parse_input("one line"));
        sheet.set(CellAddr::new(1, 0), Cell::parse_input("a\nb"));
        sheet.set(CellAddr::new(1, 5), Cell::parse_input("a\nb\nc"));
        assert_eq!(row_height(&sheet, 0, 16.0), ROW_HEIGHT);
        assert_eq!(row_height(&sheet, 1, 16.0), 3.0 * 16.0 + 2.0 * LINE_PADDING);
        assert_eq!(row_height(&sheet, 9, 16.0), ROW_HEIGHT);
    }
}
