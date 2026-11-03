//! Registers workbooks as apps in the desktop's app list and dock, each with a
//! desktop entry and a text icon, for the current user.
//!
//! Registered apps are found by their desktop entries alone, so nothing else
//! needs keeping.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Marks a desktop entry as one this command wrote.
const MARK: &str = "X-Kxcel-App";

/// Rounded-square colors, each dark enough for white text, picked by name.
const COLORS: [&str; 10] =
    ["#2f6db5", "#2e8b6e", "#b5532f", "#7a4fb5", "#b53f6d", "#3f7f8f", "#8a6d1f", "#4f5fb5", "#5f8a2e", "#9b3fa0"];

/// One registered app, as `app list` shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registered {
    pub name: String,
    pub file: PathBuf,
    pub nosave: bool,
}

/// What to register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub name: String,
    pub file: PathBuf,
    pub nosave: bool,
    /// The icon's characters. The name's first two when not given.
    pub icon: Option<String>,
}

/// The folder desktop entries and icons go under: `$XDG_DATA_HOME`, or
/// `~/.local/share`.
pub fn data_dir() -> Option<PathBuf> {
    match std::env::var_os("XDG_DATA_HOME").filter(|dir| !dir.is_empty()) {
        Some(dir) => Some(PathBuf::from(dir)),
        None => std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")),
    }
}

/// The program that runs apps: this one, or for `kxcel-cli`, the `kxcel`
/// beside it, so either can register apps.
pub fn app_program() -> io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let name = exe.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    Ok(match name.strip_suffix("-cli") {
        Some(gui) => exe.with_file_name(gui),
        None => exe,
    })
}

/// The id an app gets from its name: plain ASCII, as desktop file names must
/// be, though names may be Korean.
pub fn app_id(name: &str) -> String {
    // FNV-1a, which is stable across builds, unlike the standard hasher.
    let hash = name.bytes().fold(0x811c_9dc5_u32, |hash, byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193));
    format!("kxcel-app-{hash:08x}")
}

/// The characters on an app's icon: `icon` when given, else the first two of
/// the name that are not spaces.
pub fn icon_text(name: &str, icon: Option<&str>) -> String {
    match icon.map(str::trim).filter(|icon| !icon.is_empty()) {
        Some(icon) => icon.chars().take(3).collect(),
        None => name.chars().filter(|c| !c.is_whitespace()).take(2).collect(),
    }
}

/// The icon's background, the same for the same name.
pub fn icon_color(name: &str) -> &'static str {
    let id = app_id(name);
    let hash = u32::from_str_radix(&id["kxcel-app-".len()..], 16).unwrap_or(0);
    COLORS[hash as usize % COLORS.len()]
}

/// A square SVG icon: the text in white on a rounded square. A generic
/// sans-serif font resolves to one with Korean letters on Ubuntu.
pub fn icon_svg(text: &str, color: &str) -> String {
    let size = match text.chars().count() {
        0 | 1 => 300,
        2 => 210,
        _ => 150,
    };
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
  <rect x="16" y="16" width="480" height="480" rx="104" fill="{color}"/>
  <text x="256" y="256" fill="#ffffff" font-family="sans-serif" font-weight="bold" font-size="{size}" text-anchor="middle" dominant-baseline="central">{}</text>
</svg>
"##,
        xml_escape(text)
    )
}

/// The desktop entry for an app.
pub fn desktop_entry(request: &Request, program: &Path, id: &str) -> String {
    let mut exec = vec![
        program.to_string_lossy().into_owned(),
        "--app".into(),
        "--id".into(),
        id.into(),
        "--name".into(),
        request.name.clone(),
    ];
    if request.nosave {
        exec.push("--nosave".into());
    }
    exec.push(request.file.to_string_lossy().into_owned());
    let exec: Vec<String> = exec.iter().map(|arg| exec_arg(arg)).collect();
    let line = |value: &str| value.replace('\\', "\\\\").replace('\n', "\\n");
    format!(
        "[Desktop Entry]\nType=Application\nName={name}\nComment=A kxcel app\nExec={exec}\nIcon={id}\nTerminal=false\n\
         Categories=Office;\nStartupWMClass={id}\n{MARK}=true\nX-Kxcel-File={file}\nX-Kxcel-NoSave={nosave}\n",
        name = line(&request.name),
        exec = line(&exec.join(" ")),
        file = line(&request.file.to_string_lossy()),
        nosave = request.nosave,
    )
}

/// One `Exec` argument, quoted as the desktop entry rules ask when it holds
/// anything a shell would read, and with `%` doubled since it starts field codes.
fn exec_arg(arg: &str) -> String {
    let reserved = |c: char| c.is_whitespace() || "\"'\\><~|&;$*?#()`".contains(c);
    let arg = if arg.is_empty() || arg.contains(reserved) {
        let mut quoted = String::from("\"");
        for c in arg.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                quoted.push('\\');
            }
            quoted.push(c);
        }
        quoted.push('"');
        quoted
    } else {
        arg.to_owned()
    };
    arg.replace('%', "%%")
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn entry_path(data: &Path, id: &str) -> PathBuf {
    data.join("applications").join(format!("{id}.desktop"))
}

fn icon_path(data: &Path, id: &str) -> PathBuf {
    data.join("icons/hicolor/scalable/apps").join(format!("{id}.svg"))
}

/// Writes an app's desktop entry and icon under `data`. Returns whether it
/// replaced an app of the same name.
pub fn install(data: &Path, program: &Path, request: &Request) -> io::Result<bool> {
    let id = app_id(&request.name);
    let entry = entry_path(data, &id);
    let existed = entry.exists();
    let icon = icon_path(data, &id);
    for path in [&entry, &icon] {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
    }
    let text = icon_text(&request.name, request.icon.as_deref());
    fs::write(&icon, icon_svg(&text, icon_color(&request.name)))?;
    fs::write(&entry, desktop_entry(request, program, &id))?;
    Ok(existed)
}

/// Removes an app's desktop entry and icon. Returns false when no app has
/// that name. The workbook stays.
pub fn uninstall(data: &Path, name: &str) -> io::Result<bool> {
    let id = app_id(name);
    let entry = entry_path(data, &id);
    if !is_ours(&entry) {
        return Ok(false);
    }
    fs::remove_file(&entry)?;
    match fs::remove_file(icon_path(data, &id)) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    Ok(true)
}

fn is_ours(entry: &Path) -> bool {
    fs::read_to_string(entry).is_ok_and(|text| text.lines().any(|line| line == format!("{MARK}=true")))
}

/// The apps registered under `data`, by name.
pub fn list(data: &Path) -> Vec<Registered> {
    let Ok(dir) = fs::read_dir(data.join("applications")) else {
        return Vec::new();
    };
    let mut apps: Vec<Registered> = dir
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("kxcel-app-")))
        .filter_map(|path| fs::read_to_string(path).ok())
        .filter(|text| text.lines().any(|line| line == format!("{MARK}=true")))
        .filter_map(|text| {
            let field = |key: &str| {
                text.lines()
                    .find_map(|line| line.strip_prefix(&format!("{key}=")))
                    .map(|value| value.replace("\\n", "\n").replace("\\\\", "\\"))
            };
            Some(Registered {
                name: field("Name")?,
                file: PathBuf::from(field("X-Kxcel-File")?),
                nosave: field("X-Kxcel-NoSave").as_deref() == Some("true"),
            })
        })
        .collect();
    apps.sort_by(|a, b| a.name.cmp(&b.name));
    apps
}

/// Asks the desktop to notice changed entries and icons, as
/// `packaging/install.sh` does. Missing tools are fine.
pub fn refresh(data: &Path) {
    let quiet = |program: &str, args: &[&std::ffi::OsStr]| {
        let _ = std::process::Command::new(program)
            .args(args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    };
    quiet("update-desktop-database", &[data.join("applications").as_os_str()]);
    quiet("gtk-update-icon-cache", &["-f".as_ref(), "-t".as_ref(), data.join("icons/hicolor").as_os_str()]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(name: &str, nosave: bool, icon: Option<&str>) -> Request {
        Request {
            name: name.into(),
            file: PathBuf::from("/home/u/문서/견적 \"v2\".xlsx"),
            nosave,
            icon: icon.map(Into::into),
        }
    }

    #[test]
    fn ids_are_ascii_and_stable() {
        let id = app_id("견적 계산기");
        assert!(id.starts_with("kxcel-app-") && id.len() == "kxcel-app-".len() + 8 && id.is_ascii());
        assert_eq!(id, app_id("견적 계산기"));
        assert_ne!(id, app_id("Budget"));
    }

    #[test]
    fn icon_text_and_color() {
        assert_eq!(icon_text("견적 계산기", None), "견적");
        assert_eq!(icon_text("Budget", None), "Bu");
        assert_eq!(icon_text("A b", None), "Ab");
        assert_eq!(icon_text("Budget", Some("견")), "견");
        assert_eq!(icon_text("Budget", Some(" ")), "Bu");
        assert_eq!(icon_color("Budget"), icon_color("Budget"));
        assert!(COLORS.contains(&icon_color("견적")));
        let svg = icon_svg("<&", "#2f6db5");
        assert!(svg.contains("&lt;&amp;") && svg.contains("#2f6db5"));
    }

    #[test]
    fn desktop_entries_quote_what_needs_it() {
        let entry = desktop_entry(&request("견적 계산기", true, None), Path::new("/usr/bin/kxcel"), "kxcel-app-1");
        let exec = entry.lines().find_map(|line| line.strip_prefix("Exec=")).unwrap();
        assert_eq!(
            exec,
            r#"/usr/bin/kxcel --app --id kxcel-app-1 --name "견적 계산기" --nosave "/home/u/문서/견적 \\"v2\\".xlsx""#
        );
        assert!(entry.contains("\nName=견적 계산기\n") && entry.contains("\nIcon=kxcel-app-1\n"));
        assert!(entry.contains("\nStartupWMClass=kxcel-app-1\n"));
        assert_eq!(exec_arg("100%"), "100%%");
        assert_eq!(exec_arg(""), "\"\"");
    }

    #[test]
    fn install_list_and_uninstall() {
        let dir = tempfile::tempdir().unwrap();
        let program = Path::new("/opt/kxcel");
        assert!(!install(dir.path(), program, &request("견적 계산기", true, None)).unwrap());
        assert!(!install(dir.path(), program, &request("Budget", false, Some("$"))).unwrap());
        assert!(install(dir.path(), program, &request("Budget", false, None)).unwrap(), "again replaces it");
        let id = app_id("견적 계산기");
        let svg = fs::read_to_string(dir.path().join(format!("icons/hicolor/scalable/apps/{id}.svg"))).unwrap();
        assert!(svg.contains(">견적</text>"));
        let apps = list(dir.path());
        assert_eq!(apps.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), ["Budget", "견적 계산기"]);
        assert!(apps[1].nosave && !apps[0].nosave);
        assert_eq!(apps[1].file, PathBuf::from("/home/u/문서/견적 \"v2\".xlsx"));
        // Someone else's entry with a matching name is left alone.
        fs::write(dir.path().join("applications/kxcel-app-other.desktop"), "[Desktop Entry]\nName=x\n").unwrap();
        assert_eq!(list(dir.path()).len(), 2);
        assert!(uninstall(dir.path(), "견적 계산기").unwrap());
        assert!(!dir.path().join(format!("icons/hicolor/scalable/apps/{id}.svg")).exists());
        assert!(!uninstall(dir.path(), "견적 계산기").unwrap());
        assert_eq!(list(dir.path()).len(), 1);
    }
}
