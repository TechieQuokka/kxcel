//! egui's built-in fonts have no Korean, so a system font is added as a fallback.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use egui::{Context, FontData, FontDefinitions, FontFamily};

const FONT_NAME: &str = "system-korean";

/// Fonts to try if fontconfig cannot suggest one.
const FALLBACK_PATHS: [&str; 3] = [
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/nanum/NanumGothic.ttf",
];

/// Adds a Korean font after the built-in ones. Without one, Korean text shows
/// as boxes, but everything else still works.
pub fn install(ctx: &Context) {
    let Some(bytes) = korean_font_file().and_then(|path| std::fs::read(path).ok()) else {
        eprintln!("Warning: no Korean font found. Install fonts-noto-cjk to display Korean");
        return;
    };
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(FONT_NAME.to_owned(), Arc::new(FontData::from_owned(bytes)));
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts.families.entry(family).or_default().push(FONT_NAME.to_owned());
    }
    ctx.set_fonts(fonts);
}

fn korean_font_file() -> Option<PathBuf> {
    let from_fontconfig = Command::new("fc-match")
        .args(["-f", "%{file}", ":lang=ko"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| PathBuf::from(String::from_utf8_lossy(&output.stdout).into_owned()))
        .filter(|path| path.is_file());
    from_fontconfig.or_else(|| FALLBACK_PATHS.iter().map(PathBuf::from).find(|path| path.is_file()))
}
