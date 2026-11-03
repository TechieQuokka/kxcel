//! Grid colors that `egui::Visuals` has no slot for.

use egui::{Color32, Context, Id, Visuals};

/// Colors for tables and input cells, which the application sets for each theme with
/// [`GridPalette::store`]. Without one, shades are derived from the visuals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridPalette {
    /// Behind a table's header row.
    pub table_header: Color32,
    /// A table's outline.
    pub table_line: Color32,
    /// Behind input cells, which an app lets its users change.
    pub input_tint: Color32,
}

impl GridPalette {
    fn id() -> Id {
        Id::unique("excel_grid_palette")
    }

    /// Makes this the palette the grid draws with.
    pub fn store(self, ctx: &Context) {
        ctx.data_mut(|data| data.insert_temp(Self::id(), self));
    }

    pub(crate) fn current(ctx: &Context, visuals: &Visuals) -> Self {
        ctx.data(|data| data.get_temp(Self::id())).unwrap_or_else(|| Self::derived(visuals))
    }

    fn derived(visuals: &Visuals) -> Self {
        Self {
            table_header: visuals.selection.bg_fill.gamma_multiply(0.5),
            table_line: visuals.selection.stroke.color.gamma_multiply(0.7),
            input_tint: visuals.warn_fg_color.gamma_multiply(0.15),
        }
    }
}
