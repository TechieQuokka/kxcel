//! Command-line arguments: `kxcel [file]`, or in app mode,
//! `kxcel --app [--nosave] [--name <name>] [--id <id>] <file>`.

use std::ffi::OsString;
use std::path::PathBuf;

/// How a workbook opens as an app.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppMode {
    /// Never save, and never ask to: each start is fresh.
    pub nosave: bool,
    /// The window title. The file name when not given.
    pub name: Option<String>,
    /// The window's app id, so the desktop matches the window to the app's
    /// own entry and icon.
    pub id: Option<String>,
}

/// What to open, and how.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Launch {
    pub path: Option<PathBuf>,
    pub app: Option<AppMode>,
}

pub const USAGE: &str = "Usage: kxcel [file]\n       kxcel --app [--nosave] [--name <name>] [--id <id>] <file>";

/// Reads the arguments after the program name.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Launch, String> {
    let mut launch = Launch::default();
    let mut mode = AppMode::default();
    let (mut app, mut app_only) = (false, None);
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = |flag: &str| {
            args.next().and_then(|value| value.into_string().ok()).ok_or_else(|| format!("{flag} needs a value"))
        };
        match arg.to_str() {
            Some("--app") => app = true,
            Some("--nosave") => {
                mode.nosave = true;
                app_only = Some("--nosave");
            }
            Some("--name") => {
                mode.name = Some(value("--name")?);
                app_only = Some("--name");
            }
            Some("--id") => {
                mode.id = Some(value("--id")?);
                app_only = Some("--id");
            }
            Some(flag) if flag.starts_with("--") => return Err(format!("Unknown option {flag}")),
            _ if launch.path.is_some() => return Err("Only one file can be opened".into()),
            _ => launch.path = Some(PathBuf::from(arg)),
        }
    }
    if app {
        if launch.path.is_none() {
            return Err("--app needs a file to open".into());
        }
        launch.app = Some(mode);
    } else if let Some(flag) = app_only {
        return Err(format!("{flag} only works with --app"));
    }
    Ok(launch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(args: &[&str]) -> Result<Launch, String> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn a_plain_file_or_nothing() {
        assert_eq!(parse_strs(&[]).unwrap(), Launch::default());
        assert_eq!(parse_strs(&["a.xlsx"]).unwrap().path, Some(PathBuf::from("a.xlsx")));
    }

    #[test]
    fn app_mode_with_options_in_any_order() {
        let launch =
            parse_strs(&["--name", "견적 계산기", "--app", "q.xlsx", "--nosave", "--id", "kxcel-app-1"]).unwrap();
        assert_eq!(launch.path, Some(PathBuf::from("q.xlsx")));
        let mode = launch.app.unwrap();
        assert!(mode.nosave);
        assert_eq!(mode.name.as_deref(), Some("견적 계산기"));
        assert_eq!(mode.id.as_deref(), Some("kxcel-app-1"));
    }

    #[test]
    fn mistakes_are_explained() {
        assert!(parse_strs(&["--app"]).unwrap_err().contains("needs a file"));
        assert!(parse_strs(&["--nosave", "a.xlsx"]).unwrap_err().contains("only works with --app"));
        assert!(parse_strs(&["--app", "a.xlsx", "--name"]).unwrap_err().contains("needs a value"));
        assert!(parse_strs(&["--bogus"]).unwrap_err().contains("Unknown option"));
        assert!(parse_strs(&["a.xlsx", "b.xlsx"]).is_err());
    }
}
