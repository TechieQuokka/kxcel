use egui::{Align2, FontId, Rect, Stroke, StrokeKind, Ui, pos2, vec2};
use excel_model::col_name;

use crate::GridState;
use crate::layout::{COL_HEADER_HEIGHT, Layout, ROW_HEADER_WIDTH};

/// Draws the row and column headers, which stay fixed while cells scroll.
/// With `filtered`, row numbers take the accent color, as a hint that some
/// rows are hidden; the numbers themselves stay the real ones.
pub(crate) fn paint(ui: &Ui, state: &GridState, layout: &Layout, filtered: bool) {
    let visuals = ui.visuals();
    let painter = ui.painter().with_clip_rect(layout.rect);
    let line = Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color);
    let fill = visuals.faint_bg_color;
    let on_fill = visuals.selection.bg_fill;
    let text = visuals.weak_text_color();
    let on_text = visuals.strong_text_color();
    let font = FontId::monospace(12.0);
    let selection = state.selection();

    let top = Rect::from_min_size(layout.rect.min, vec2(layout.rect.width(), COL_HEADER_HEIGHT));
    let left = Rect::from_min_size(layout.rect.min, vec2(ROW_HEADER_WIDTH, layout.rect.height()));
    painter.rect_filled(top, 0.0, fill);
    painter.rect_filled(left, 0.0, fill);

    let col_painter = painter.with_clip_rect(top.intersect(layout.rect));
    for span in &layout.cols {
        let cell = Rect::from_min_size(pos2(span.x, top.top()), vec2(span.width, COL_HEADER_HEIGHT));
        let on = (selection.start.col..=selection.end.col).contains(&span.col);
        if on {
            col_painter.rect_filled(cell, 0.0, on_fill);
        }
        col_painter.rect_stroke(cell, 0.0, line, StrokeKind::Inside);
        col_painter.text(
            cell.center(),
            Align2::CENTER_CENTER,
            col_name(span.col),
            font.clone(),
            if on { on_text } else { text },
        );
    }

    let row_painter = painter.with_clip_rect(left.intersect(layout.rect));
    for span in &layout.rows {
        let row = span.row;
        let cell = Rect::from_min_size(pos2(left.left(), span.y), vec2(ROW_HEADER_WIDTH, span.height));
        let on = (selection.start.row..=selection.end.row).contains(&row);
        if on {
            row_painter.rect_filled(cell, 0.0, on_fill);
        }
        row_painter.rect_stroke(cell, 0.0, line, StrokeKind::Inside);
        let color = if filtered {
            visuals.selection.stroke.color
        } else if on {
            on_text
        } else {
            text
        };
        row_painter.text(cell.center(), Align2::CENTER_CENTER, row + 1, font.clone(), color);
    }

    let corner = Rect::from_min_size(layout.rect.min, vec2(ROW_HEADER_WIDTH, COL_HEADER_HEIGHT));
    painter.rect_filled(corner, 0.0, fill);
    painter.rect_stroke(corner, 0.0, line, StrokeKind::Inside);
}
