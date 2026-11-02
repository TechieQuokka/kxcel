//! Settings kept between runs, in a small `key = value` file under the user's
//! configuration folder. Only the theme is kept for now.

use std::path::{Path, PathBuf};

use excel_command::Theme;

/// What the application remembers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Settings {
    pub theme: Theme,
}

impl Settings {
    /// Reads settings. A missing or unreadable file, or an unknown value,
    /// leaves the default.
    pub fn load(path: &Path) -> Self {
        let mut settings = Self::default();
        let Ok(text) = std::fs::read_to_string(path) else {
            return settings;
        };
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key.trim() == "theme"
                && let Some(theme) = Theme::from_name(value)
            {
                settings.theme = theme;
            }
        }
        settings
    }

    /// Writes settings, creating the folder if needed.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(path, format!("theme = {}\n", self.theme.name()))
    }
}

/// Where settings live: `$XDG_CONFIG_HOME/kxcel/settings.conf`, or
/// `~/.config/kxcel/settings.conf`. Tests get none, so they never touch the
/// user's settings.
pub fn default_path() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(config.join("kxcel").join("settings.conf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_the_theme() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kxcel").join("settings.conf");
        assert_eq!(Settings::load(&path), Settings::default(), "a missing file gives defaults");
        Settings { theme: Theme::Light }.save(&path).unwrap();
        assert_eq!(Settings::load(&path).theme, Theme::Light);
    }

    #[test]
    fn ignores_what_it_does_not_know() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.conf");
        std::fs::write(&path, "# note\ncolor = red\ntheme = purple\nnot a setting\n").unwrap();
        assert_eq!(Settings::load(&path).theme, Theme::Dark);
        std::fs::write(&path, " theme =  LIGHT \n").unwrap();
        assert_eq!(Settings::load(&path).theme, Theme::Light);
    }

    #[test]
    fn tests_never_use_the_real_settings() {
        assert_eq!(default_path(), None);
    }
}
