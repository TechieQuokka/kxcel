//! Builds the help panel's contents from the registries, so new commands and
//! functions appear without changes here.

use excel_command::{CommandRegistry, Doc, Theme};
use excel_features::{COMMAND_CATEGORIES, FUNCTION_CATEGORIES};
use excel_ui_help_panel::{HelpEntry, HelpSection};

/// Keyboard and mouse shortcuts. Keep in step with the grid, command line, and keymap.
const KEYS: [(&str, &str); 22] = [
    ("F1", "Show or hide this help"),
    ("F11", "Full screen on or off"),
    ("Ctrl+L", "Go to the command line"),
    ("Esc", "Back to the grid from the command line or help search, or cancel an edit"),
    ("Tab", "Complete a command, path, or function, or close one open parenthesis"),
    ("Ctrl+S", "Save, same as the save command"),
    ("Enter", "Finish an edit and move down"),
    ("Alt+Enter", "New line inside a cell"),
    ("F2", "Edit the current cell"),
    ("Delete", "Clear the selection"),
    ("Ctrl+Z", "Undo"),
    ("Ctrl+Y", "Redo"),
    ("Ctrl+C / X / V", "Copy, cut, paste"),
    ("Shift+Arrows", "Extend the selection"),
    ("Ctrl+Arrows", "Jump to the edge of the data"),
    ("Ctrl+Shift+Arrows", "Extend the selection to the edge of the data"),
    ("Ctrl+T", "Make the selection, or the data around the cursor, a table"),
    ("Drag corner", "Fill: continue 1, 2, 3 or 1, 2, 4, else copy"),
    ("Ctrl+drag corner", "Fill: count up a single number, or copy a series"),
    ("Drag corner back", "Clear the cells left out"),
    ("Double-click", "Header edge: fit the column width"),
    ("Drag in a formula", "Insert a range reference"),
];

/// The help panel's tabs: functions are typed in cells, and commands and keys
/// work the application.
pub const TABS: [&str; 2] = ["Functions", "Commands"];

/// Functions show with their `=` and their own color. Commands and keys share
/// the second tab, with keys as one group after the command groups.
pub fn sections(registry: &CommandRegistry, functions: &[(&'static str, Doc)], theme: Theme) -> Vec<HelpSection> {
    let entries = functions.iter().map(|(name, doc)| HelpEntry::new(*name, *doc)).collect();
    let mut functions = HelpSection::grouped("Type = in a cell", entries, FUNCTION_CATEGORIES);
    functions.prefix = "=".into();
    functions.color = Some(crate::theme::function_color(theme));

    let commands = registry.docs().into_iter().map(|(name, doc)| HelpEntry::new(name, doc)).collect();
    let mut commands = HelpSection::grouped("Press Ctrl+L, then type", commands, COMMAND_CATEGORIES);
    commands.tab = 1;

    let keys = KEYS
        .iter()
        .map(|(key, summary)| HelpEntry::new(*key, Doc { category: "Keys", summary, usage: key, ..Doc::BLANK }))
        .collect();
    let mut keys = HelpSection::grouped("", keys, &[]);
    keys.tab = 1;

    vec![functions, commands, keys]
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
        let sections = sections(&excel_features::registry(), &functions, Theme::Dark);
        let [functions, commands, keys] = &sections[..] else { panic!("three sections") };
        for name in ["help", "sort", "save", "save!", "exit", "open"] {
            assert!(names(commands).contains(&name), "{name}");
        }
        assert_eq!(commands.groups[0].title, "File");
        assert!(names(functions).contains(&"SUM"));
        assert_eq!(functions.prefix, "=");
        assert!(commands.title.contains("Ctrl+L") && functions.title.contains('='));
        assert_ne!(commands.color, functions.color, "commands and functions look different");
        assert_eq!((functions.tab, commands.tab, keys.tab), (0, 1, 1), "functions, then commands with keys");
        assert_eq!(keys.groups.len(), 1, "keys are one group");
        assert_eq!(names(keys).len(), KEYS.len());
    }

    #[test]
    fn every_entry_has_a_summary_and_usage_and_known_related_names() {
        let functions = excel_features::Services::default().calculator.functions();
        let sections = sections(&excel_features::registry(), &functions, Theme::Dark);
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
