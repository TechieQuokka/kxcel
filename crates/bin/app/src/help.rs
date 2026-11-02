//! Builds the help panel's contents from the registries, so new commands and
//! functions appear without changes here.

use excel_command::CommandRegistry;
use excel_ui_grid::FunctionInfo;
use excel_ui_help_panel::{HelpEntry, HelpSection};

/// Keyboard and mouse shortcuts. Keep in step with the grid, command line, and keymap.
const KEYS: [(&str, &str); 16] = [
    ("F1", "Show or hide this help"),
    ("Ctrl+L", "Go to the command line"),
    ("Esc", "Back to the grid, or cancel an edit"),
    ("Tab", "Complete a command or function"),
    ("Enter", "Finish an edit and move down"),
    ("F2", "Edit the current cell"),
    ("Delete", "Clear the selection"),
    ("Ctrl+Z", "Undo"),
    ("Ctrl+Y", "Redo"),
    ("Ctrl+C / X / V", "Copy, cut, paste"),
    ("Ctrl+S", "Save, same as w"),
    ("Shift+Arrows", "Extend the selection"),
    ("Drag corner", "Fill: continue 1, 2, 3 or 1, 2, 4, else copy"),
    ("Ctrl+drag corner", "Fill: count up a single number, or copy a series"),
    ("Drag corner back", "Clear the cells left out"),
    ("Double-click", "Header edge: fit the column width"),
];

/// Commands and functions are typed in different places, so the titles say
/// where, and functions show with their `=` and their own color.
pub fn sections(registry: &CommandRegistry, functions: &[FunctionInfo]) -> Vec<HelpSection> {
    let commands = registry.complete("").into_iter().map(|c| HelpEntry::new(c.name, c.help)).collect();
    let functions = functions.iter().map(function_entry).collect();
    let keys = KEYS.iter().map(|(key, usage)| HelpEntry::new(*key, *usage)).collect();
    vec![
        HelpSection { title: "Commands: Ctrl+L, then type".into(), entries: commands, color: None },
        HelpSection {
            title: "Functions: type = in a cell".into(),
            entries: functions,
            color: Some(crate::theme::FUNCTION_COLOR),
        },
        HelpSection { title: "Keys".into(), entries: keys, color: None },
    ]
}

/// Splits a function's help, such as `SUM(values...)  Add numbers`, into
/// `=SUM(values...)` and `Add numbers`.
fn function_entry(function: &FunctionInfo) -> HelpEntry {
    match function.help.split_once("  ") {
        Some((signature, description)) => HelpEntry::new(format!("={signature}"), description.trim()),
        None => HelpEntry::new(format!("={}", function.name), function.help.trim()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_every_command_and_function() {
        let functions = vec![FunctionInfo { name: "SUM".into(), help: "SUM(values...)  Add numbers".into() }];
        let sections = sections(&excel_features::registry(), &functions);
        let names: Vec<&str> = sections[0].entries.iter().map(|e| e.name.as_str()).collect();
        for name in ["help", "sort", "w", "w!"] {
            assert!(names.contains(&name), "{name}");
        }
        assert_eq!(sections[1].entries, [HelpEntry::new("=SUM(values...)", "Add numbers")]);
        assert!(sections[0].title.contains("Ctrl+L") && sections[1].title.contains('='));
        assert_ne!(sections[0].color, sections[1].color, "commands and functions look different");
        assert!(!sections[2].entries.is_empty());
    }
}
