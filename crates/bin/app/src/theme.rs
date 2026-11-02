//! The one place application colors are defined. Widgets read them from
//! `egui::Visuals`, so each theme is one palette here.

use egui::{Color32, Context, Stroke, Visuals};
use excel_command::Theme;

/// Function names in help, set apart from command names, which use the accent
/// color. Light backgrounds need a darker shade.
pub fn function_color(theme: Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgb(196, 168, 255),
        Theme::Light => Color32::from_rgb(112, 64, 196),
    }
}

/// The colors a theme sets on top of egui's own dark or light look.
struct Palette {
    base: fn() -> Visuals,
    accent: Color32,
    line: Color32,
    panel: Color32,
    extreme: Color32,
    faint: Color32,
    text: Color32,
    selection: Color32,
    error: Color32,
    warn: Color32,
}

const DARK: Palette = Palette {
    base: Visuals::dark,
    accent: Color32::from_rgb(90, 169, 230),
    line: Color32::from_rgb(54, 54, 54),
    panel: Color32::from_rgb(36, 36, 36),
    extreme: Color32::from_rgb(27, 27, 27),
    faint: Color32::from_rgb(44, 44, 44),
    text: Color32::from_rgb(214, 214, 214),
    selection: Color32::from_rgb(58, 74, 87),
    error: Color32::from_rgb(232, 132, 132),
    warn: Color32::from_rgb(224, 184, 110),
};

const LIGHT: Palette = Palette {
    base: Visuals::light,
    accent: Color32::from_rgb(0, 106, 180),
    line: Color32::from_rgb(214, 214, 214),
    panel: Color32::from_rgb(246, 246, 246),
    extreme: Color32::from_rgb(255, 255, 255),
    faint: Color32::from_rgb(236, 236, 236),
    text: Color32::from_rgb(36, 36, 36),
    selection: Color32::from_rgb(204, 226, 244),
    error: Color32::from_rgb(190, 40, 40),
    warn: Color32::from_rgb(150, 100, 0),
};

/// Applies a theme to every widget.
pub fn apply(ctx: &Context, theme: Theme) {
    let palette = match theme {
        Theme::Dark => &DARK,
        Theme::Light => &LIGHT,
    };
    let mut visuals = (palette.base)();
    visuals.panel_fill = palette.panel;
    visuals.extreme_bg_color = palette.extreme;
    visuals.faint_bg_color = palette.faint;
    visuals.override_text_color = Some(palette.text);
    visuals.selection.bg_fill = palette.selection;
    visuals.selection.stroke = Stroke::new(1.0, palette.accent);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.line);
    visuals.error_fg_color = palette.error;
    visuals.warn_fg_color = palette.warn;
    ctx.set_visuals(visuals);
}
