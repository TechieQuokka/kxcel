//! Builds the help panel's contents from the registries, so new commands and
//! functions appear without changes here.

use excel_command::{CommandRegistry, Doc};
use excel_features::{COMMAND_CATEGORIES, FUNCTION_CATEGORIES};
use excel_ui_help_panel::{HelpEntry, HelpSection};

/// Keyboard and mouse shortcuts. Keep in step with the grid, command line, and keymap.
const KEYS: [(&str, &str, &str); 17] = [
    ("F1", "Show or hide this help", "App"),
    ("Ctrl+L", "Go to the command line", "App"),
    ("Esc", "Back to the grid, or cancel an edit", "App"),
    ("Tab", "Complete a command, path, or function", "App"),
    ("Ctrl+S", "Save, same as the save command", "App"),
    ("Enter", "Finish an edit and move down", "Editing"),
    ("Alt+Enter", "New line inside a cell", "Editing"),
    ("F2", "Edit the current cell", "Editing"),
    ("Delete", "Clear the selection", "Editing"),
    ("Ctrl+Z", "Undo", "Editing"),
    ("Ctrl+Y", "Redo", "Editing"),
    ("Ctrl+C / X / V", "Copy, cut, paste", "Editing"),
    ("Shift+Arrows", "Extend the selection", "Selection and fill"),
    ("Drag corner", "Fill: continue 1, 2, 3 or 1, 2, 4, else copy", "Selection and fill"),
    ("Ctrl+drag corner", "Fill: count up a single number, or copy a series", "Selection and fill"),
    ("Drag corner back", "Clear the cells left out", "Selection and fill"),
    ("Double-click", "Header edge: fit the column width", "Selection and fill"),
];

/// Commands and functions are typed in different places, so the titles say
/// where, and functions show with their `=` and their own color.
pub fn sections(registry: &CommandRegistry, functions: &[(&'static str, Doc)]) -> Vec<HelpSection> {
    let commands = registry.docs().into_iter().map(|(name, doc)| HelpEntry::new(name, doc)).collect();
    let commands = HelpSection::grouped("Commands: Ctrl+L, then type", commands, COMMAND_CATEGORIES);

    let entries = functions.iter().map(|(name, doc)| HelpEntry::new(*name, *doc)).collect();
    let mut functions = HelpSection::grouped("Functions: type = in a cell", entries, FUNCTION_CATEGORIES);
    functions.prefix = "=".into();
    functions.color = Some(crate::theme::FUNCTION_COLOR);

    let keys = KEYS
        .iter()
        .map(|(key, summary, category)| HelpEntry::new(*key, Doc { category, summary, usage: key, ..Doc::BLANK }))
        .collect();
    let keys = HelpSection::grouped("Keys", keys, &["App", "Editing", "Selection and fill"]);

    vec![commands, functions, keys]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(section: &HelpSection) -> Vec<&str> {
        section.groups.iter().flat_map(|g| g.entries.iter().map(|e| e.name.as_str())).collect()
    }

    #[test]
    fn lists_every_command_and_function_by_category() {
        let functions = excel_features::Services::default().calculator.functions();
        let sections = sections(&excel_features::registry(), &functions);
        for name in ["help", "sort", "save", "save!", "exit", "open"] {
            assert!(names(&sections[0]).contains(&name), "{name}");
        }
        assert_eq!(sections[0].groups[0].title, "File");
        assert!(names(&sections[1]).contains(&"SUM"));
        assert_eq!(sections[1].prefix, "=");
        assert!(sections[0].title.contains("Ctrl+L") && sections[1].title.contains('='));
        assert_ne!(sections[0].color, sections[1].color, "commands and functions look different");
        assert_eq!(names(&sections[2]).len(), KEYS.len());
    }

    #[test]
    fn every_entry_has_a_summary_and_usage_and_known_related_names() {
        let functions = excel_features::Services::default().calculator.functions();
        let sections = sections(&excel_features::registry(), &functions);
        for (index, section) in sections.iter().enumerate() {
            for entry in section.groups.iter().flat_map(|g| &g.entries) {
                assert!(!entry.doc.summary.is_empty() && !entry.doc.usage.is_empty(), "{}", entry.name);
                assert_ne!(entry.doc.category, "Other", "{} needs a category", entry.name);
                for related in entry.doc.related {
                    assert!(
                        excel_ui_help_panel::find(&sections, related, Some(index)).is_some(),
                        "{} names unknown {related}",
                        entry.name
                    );
                }
            }
        }
    }
}
