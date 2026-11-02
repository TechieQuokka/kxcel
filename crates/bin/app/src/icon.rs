//! The window icon, from `assets/icon.svg`.
//!
//! It is embedded as raw RGBA so no image decoder is needed. On Wayland, the
//! desktop shows the icon from the installed desktop entry instead, which
//! `packaging/install.sh` sets up.

use egui::IconData;

const SIZE: u32 = 256;
const RGBA: &[u8] = include_bytes!("../../../../assets/icons/kxcel-256.rgba");

pub fn window_icon() -> IconData {
    IconData { rgba: RGBA.to_vec(), width: SIZE, height: SIZE }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_has_one_rgba_pixel_per_position() {
        let icon = window_icon();
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
    }

    #[test]
    fn the_corner_is_transparent_and_the_center_is_not() {
        let icon = window_icon();
        let alpha = |x: u32, y: u32| icon.rgba[((y * SIZE + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0);
        assert_eq!(alpha(SIZE / 2, SIZE / 2), 255);
    }
}
