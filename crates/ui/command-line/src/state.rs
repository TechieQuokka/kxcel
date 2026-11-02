use excel_command::{Candidate, CommandRegistry};

/// The command line's text, completion, and latest result.
#[derive(Debug, Default)]
pub struct CommandLineState {
    pub(crate) text: String,
    pub(crate) picked: usize,
    pub(crate) candidates: Vec<Candidate>,
    pub(crate) cursor_to_end: bool,
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

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Recomputes candidates. Completion covers the command name only, so it
    /// stops once an argument has started.
    pub(crate) fn refresh(&mut self, registry: &CommandRegistry) {
        let name = self.text.trim_start();
        self.candidates =
            if name.is_empty() || name.contains(char::is_whitespace) { Vec::new() } else { registry.complete(name) };
        self.picked = self.picked.min(self.candidates.len().saturating_sub(1));
    }

    pub(crate) fn pick_step(&mut self, step: isize) {
        let len = self.candidates.len() as isize;
        if len > 0 {
            self.picked = (self.picked as isize + step).rem_euclid(len) as usize;
        }
    }

    /// Replaces the text with the picked candidate. Returns false without one.
    pub(crate) fn complete(&mut self) -> bool {
        let Some(candidate) = self.candidates.get(self.picked) else {
            return false;
        };
        self.text = format!("{} ", candidate.name);
        self.cursor_to_end = true;
        self.picked = 0;
        self.candidates.clear();
        true
    }

    pub(crate) fn take_line(&mut self) -> String {
        self.picked = 0;
        self.candidates.clear();
        std::mem::take(&mut self.text)
    }
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
        fn help(&self) -> &str {
            "help"
        }
        fn run(&self, _: &mut Context<'_>, _: &[&str]) -> Result<(), CommandError> {
            Ok(())
        }
    }

    fn registry() -> CommandRegistry {
        let mut registry = CommandRegistry::default();
        registry.register(Named("sort"));
        registry.register(Named("sheet"));
        registry
    }

    fn typed(text: &str) -> CommandLineState {
        let mut state = CommandLineState { text: text.into(), ..Default::default() };
        state.refresh(&registry());
        state
    }

    #[test]
    fn empty_text_has_no_candidates() {
        assert!(typed("").candidates.is_empty());
    }

    #[test]
    fn arguments_end_completion() {
        assert_eq!(typed("s").candidates.len(), 2);
        assert!(typed("sort ").candidates.is_empty());
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
        assert!(state.candidates.is_empty());
        assert!(!state.complete());
    }

    #[test]
    fn taking_the_line_clears_it() {
        let mut state = typed("sort B");
        assert_eq!(state.take_line(), "sort B");
        assert_eq!(state.text, "");
    }
}
