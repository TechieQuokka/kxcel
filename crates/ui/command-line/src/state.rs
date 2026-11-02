use excel_command::path::complete_path;
use excel_command::{CommandRegistry, quote_arg};

/// What the open completion list offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Completing {
    #[default]
    Commands,
    Paths,
}

/// One completion choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Item {
    pub label: String,
    pub help: String,
    /// The whole command line after picking this item.
    pub line: String,
}

/// The command line's text, completion, and latest result.
#[derive(Debug, Default)]
pub struct CommandLineState {
    pub(crate) text: String,
    pub(crate) picked: usize,
    pub(crate) items: Vec<Item>,
    pub(crate) completing: Completing,
    pub(crate) cursor_to_end: bool,
    /// Set when the pick moves, so the list scrolls to it once.
    pub(crate) scroll_to_pick: bool,
    pub(crate) message: Option<(String, bool)>,
}

impl CommandLineState {
    pub fn set_message(&mut self, text: impl Into<String>) {
        self.message = Some((text.into(), false));
    }

    pub fn set_error(&mut self, text: impl Into<String>) {
        self.message = Some((text.into(), true));
    }

    pub fn clear_message(&mut self) {
        self.message = None;
    }

    /// The message showing, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_ref().map(|(text, _)| text.as_str())
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the text, with the cursor at its end. The message stays.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor_to_end = true;
    }

    /// The labels on offer, for tests and the popup.
    pub fn offered(&self) -> Vec<&str> {
        self.items.iter().map(|item| item.label.as_str()).collect()
    }

    /// Recomputes the choices: command names while the name is typed, then
    /// files and folders for a command that takes a path.
    pub(crate) fn refresh(&mut self, registry: &CommandRegistry) {
        let line = self.text.trim_start();
        let (name, rest) = match line.find(char::is_whitespace) {
            Some(end) => (&line[..end], Some(line[end..].trim_start())),
            None => (line, None),
        };
        (self.completing, self.items) = match rest {
            None if name.is_empty() => (Completing::Commands, Vec::new()),
            None => (Completing::Commands, command_items(registry, name)),
            Some(arg) if registry.takes_path(name) => (Completing::Paths, path_items(name, arg)),
            Some(_) => (Completing::Commands, Vec::new()),
        };
        self.picked = self.picked.min(self.items.len().saturating_sub(1));
    }

    pub(crate) fn pick_step(&mut self, step: isize) {
        let len = self.items.len() as isize;
        if len > 0 {
            self.picked = (self.picked as isize + step).rem_euclid(len) as usize;
            self.scroll_to_pick = true;
        }
    }

    /// Replaces the text with the picked choice. Returns false without one.
    pub(crate) fn complete(&mut self) -> bool {
        let Some(item) = self.items.get(self.picked) else {
            return false;
        };
        self.text = item.line.clone();
        self.cursor_to_end = true;
        self.picked = 0;
        self.items.clear();
        true
    }

    pub(crate) fn take_line(&mut self) -> String {
        self.picked = 0;
        self.items.clear();
        std::mem::take(&mut self.text)
    }
}

fn command_items(registry: &CommandRegistry, name: &str) -> Vec<Item> {
    registry
        .complete(name)
        .into_iter()
        .map(|candidate| Item {
            line: format!("{} ", candidate.name),
            label: if candidate.usage.is_empty() { candidate.name } else { candidate.usage },
            help: candidate.help,
        })
        .collect()
}

/// Files and folders completing `arg`, the text typed after `command`. A path
/// with spaces is quoted, and a folder's quote stays open to keep typing.
fn path_items(command: &str, arg: &str) -> Vec<Item> {
    let typed = match arg.strip_prefix('"') {
        // A closed quote means the path is finished.
        Some(inner) if inner.contains('"') => return Vec::new(),
        Some(inner) => inner,
        None if arg.contains(char::is_whitespace) => return Vec::new(),
        None => arg,
    };
    let found = complete_path(typed);
    // A file typed out in full needs no list.
    if let [only] = found.as_slice()
        && !only.is_dir
        && only.completed == typed
    {
        return Vec::new();
    }
    found
        .into_iter()
        .map(|candidate| {
            let quoted = quote_arg(&candidate.completed);
            let arg =
                if candidate.is_dir && quoted.ends_with('"') { quoted[..quoted.len() - 1].to_owned() } else { quoted };
            Item {
                line: format!("{command} {arg}"),
                help: if candidate.is_dir { "folder".into() } else { String::new() },
                label: candidate.name,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use excel_command::{Command, CommandError, Context};

    use super::*;

    struct Named(&'static str);

    impl Command for Named {
        fn name(&self) -> &str {
            self.0
        }
        fn doc(&self) -> excel_command::Doc {
            excel_command::Doc { summary: "help", ..excel_command::Doc::BLANK }
        }
        fn run(&self, _: &mut Context<'_>, _: &[&str]) -> Result<(), CommandError> {
            Ok(())
        }
    }

    struct Opens;

    impl Command for Opens {
        fn name(&self) -> &str {
            "open"
        }
        fn doc(&self) -> excel_command::Doc {
            excel_command::Doc { summary: "Open a file", ..excel_command::Doc::BLANK }
        }
        fn run(&self, _: &mut Context<'_>, _: &[&str]) -> Result<(), CommandError> {
            Ok(())
        }
        fn takes_path(&self) -> bool {
            true
        }
    }

    fn registry() -> CommandRegistry {
        let mut registry = CommandRegistry::default();
        registry.register(Named("sort"));
        registry.register(Named("sheet"));
        registry.register(Opens);
        registry
    }

    fn folder() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("data.xlsx"), "").unwrap();
        std::fs::write(dir.path().join("my book.xlsx"), "").unwrap();
        std::fs::create_dir(dir.path().join("sub dir")).unwrap();
        let base = dir.path().display().to_string();
        (dir, base)
    }

    #[test]
    fn a_path_command_offers_files_after_its_name() {
        let (_dir, base) = folder();
        let state = typed(&format!("open {base}/"));
        assert_eq!(state.completing, Completing::Paths);
        assert_eq!(state.offered(), ["sub dir/", "data.xlsx", "my book.xlsx"]);
        assert!(typed(&format!("sort {base}/")).items.is_empty(), "sort takes no path");
    }

    #[test]
    fn completing_a_path_quotes_spaces_and_keeps_folders_open() {
        let (_dir, base) = folder();
        let mut state = typed(&format!("open {base}/m"));
        assert!(state.complete());
        assert_eq!(state.text, format!("open \"{base}/my book.xlsx\""));
        state.refresh(&registry());
        assert!(state.items.is_empty(), "a closed quote ends completion");

        let mut state = typed(&format!("open {base}/s"));
        assert!(state.complete());
        assert_eq!(state.text, format!("open \"{base}/sub dir/"));
        state.refresh(&registry());
        assert_eq!(state.completing, Completing::Paths, "the folder's contents follow");
    }

    #[test]
    fn a_finished_file_name_closes_the_list() {
        let (_dir, base) = folder();
        let mut state = typed(&format!("open {base}/d"));
        assert!(state.complete());
        assert_eq!(state.text, format!("open {base}/data.xlsx"));
        state.refresh(&registry());
        assert!(state.items.is_empty());
    }

    fn typed(text: &str) -> CommandLineState {
        let mut state = CommandLineState { text: text.into(), ..Default::default() };
        state.refresh(&registry());
        state
    }

    #[test]
    fn empty_text_has_no_candidates() {
        assert!(typed("").items.is_empty());
    }

    #[test]
    fn arguments_end_completion() {
        assert_eq!(typed("s").items.len(), 2);
        assert!(typed("sort ").items.is_empty());
    }

    #[test]
    fn picking_wraps_around() {
        let mut state = typed("s");
        state.pick_step(-1);
        assert_eq!(state.picked, 1);
        state.pick_step(1);
        assert_eq!(state.picked, 0);
    }

    #[test]
    fn completing_leaves_room_for_arguments() {
        let mut state = typed("so");
        assert!(state.complete());
        assert_eq!(state.text, "sort ");
        assert!(state.items.is_empty());
        assert!(!state.complete());
    }

    #[test]
    fn taking_the_line_clears_it() {
        let mut state = typed("sort B");
        assert_eq!(state.take_line(), "sort B");
        assert_eq!(state.text, "");
    }
}
