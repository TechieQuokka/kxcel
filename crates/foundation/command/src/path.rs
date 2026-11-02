//! File paths typed on the command line: `~` expansion and completion.

use std::path::{Path, PathBuf};

/// The file types the file commands read and write, as lowercase extensions.
pub const FILE_EXTENSIONS: &[&str] = &["xlsx", "csv"];

/// The most candidates completion offers, so a huge folder stays quick.
const MAX_CANDIDATES: usize = 200;

/// Expands a leading `~` to the home folder, as a shell does.
pub fn expand_home(path: &str) -> PathBuf {
    let home = || std::env::var_os("HOME").map(PathBuf::from);
    if path == "~" {
        if let Some(home) = home() {
            return home;
        }
    } else if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = home()
    {
        return home.join(rest);
    }
    PathBuf::from(path)
}

/// A folder or file that completes a typed path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathCandidate {
    /// The entry's own name, with `/` after a folder.
    pub name: String,
    /// The whole path as typed so far, ending with this entry.
    pub completed: String,
    pub is_dir: bool,
}

/// Folders and supported files whose names start with the last part of
/// `typed`, ignoring case. Folders come first. Hidden entries are offered only
/// once a `.` is typed.
pub fn complete_path(typed: &str) -> Vec<PathCandidate> {
    let (folder, prefix) = match typed.rfind('/') {
        Some(slash) => typed.split_at(slash + 1),
        None => ("", typed),
    };
    let listed = if folder.is_empty() { PathBuf::from(".") } else { expand_home(folder) };
    let Ok(entries) = std::fs::read_dir(&listed) else {
        return Vec::new();
    };
    let prefix_lower = prefix.to_lowercase();

    let mut found: Vec<PathCandidate> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            if name.starts_with('.') && !prefix.starts_with('.') {
                return None;
            }
            if !name.to_lowercase().starts_with(&prefix_lower) {
                return None;
            }
            // Follows links, so a link to a folder counts as a folder.
            let is_dir = entry.path().is_dir();
            if !is_dir && !is_supported(Path::new(&name)) {
                return None;
            }
            let name = if is_dir { format!("{name}/") } else { name };
            Some(PathCandidate { completed: format!("{folder}{name}"), name, is_dir })
        })
        .collect();
    found.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    found.truncate(MAX_CANDIDATES);
    found
}

/// Whether the file commands can read this file, judged by its extension.
pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| FILE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(found: &[PathCandidate]) -> Vec<&str> {
        found.iter().map(|c| c.name.as_str()).collect()
    }

    fn folder() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for file in ["data.xlsx", "Draft.XLSX", "notes.txt", ".hidden.xlsx", "매출.xlsx", "list.csv"] {
            std::fs::write(dir.path().join(file), "").unwrap();
        }
        std::fs::create_dir(dir.path().join("docs")).unwrap();
        std::fs::create_dir(dir.path().join("my folder")).unwrap();
        dir
    }

    #[test]
    fn lists_folders_first_then_supported_files() {
        let dir = folder();
        let typed = format!("{}/", dir.path().display());
        let found = complete_path(&typed);
        assert_eq!(names(&found), ["docs/", "my folder/", "data.xlsx", "Draft.XLSX", "list.csv", "매출.xlsx"]);
        assert_eq!(found[0].completed, format!("{typed}docs/"));
        assert!(found[0].is_dir && !found[2].is_dir);
    }

    #[test]
    fn narrows_by_prefix_ignoring_case() {
        let dir = folder();
        let base = dir.path().display();
        assert_eq!(names(&complete_path(&format!("{base}/d"))), ["docs/", "data.xlsx", "Draft.XLSX"]);
        assert_eq!(names(&complete_path(&format!("{base}/DA"))), ["data.xlsx"]);
        assert_eq!(names(&complete_path(&format!("{base}/매"))), ["매출.xlsx"]);
    }

    #[test]
    fn hidden_entries_need_a_dot() {
        let dir = folder();
        assert_eq!(names(&complete_path(&format!("{}/.", dir.path().display()))), [".hidden.xlsx"]);
    }

    #[test]
    fn a_missing_folder_offers_nothing() {
        assert!(complete_path("/nonexistent/folder/x").is_empty());
    }

    #[test]
    fn expands_the_home_folder() {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap());
        assert_eq!(expand_home("~/a.xlsx"), home.join("a.xlsx"));
        assert_eq!(expand_home("~"), home);
        assert_eq!(expand_home("a~/b"), PathBuf::from("a~/b"));
    }
}
