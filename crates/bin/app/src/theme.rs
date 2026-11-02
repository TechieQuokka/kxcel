//! The one place application colors are defined. Widgets read them from
//! `egui::Visuals`, so adding a light theme only needs a second palette here.

use egui::{Color32, Context, Stroke, Visuals};

/// Function names in help and completion, set apart from command names, which
/// use the accent color.
pub const FUNCTION_COLOR: Color32 = Color32::from_rgb(196, 168, 255);

/// Applies the dark theme.
pub fn apply(ctx: &Context) {
    let accent = Color32::from_rgb(90, 169, 230);
    let line = Color32::from_rgb(54, 54, 54);

    let mut visuals = Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(36, 36, 36);
    visuals.extreme_bg_color = Color32::from_rgb(27, 27, 27);
    visuals.faint_bg_color = Color32::from_rgb(44, 44, 44);
    visuals.override_text_color = Some(Color32::from_rgb(214, 214, 214));
    visuals.selection.bg_fill = Color32::from_rgb(58, 74, 87);
    visuals.selection.stroke = Stroke::new(1.0, accent);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, line);
    visuals.error_fg_color = Color32::from_rgb(232, 132, 132);
    visuals.warn_fg_color = Color32::from_rgb(224, 184, 110);
    ctx.set_visuals(visuals);
}
