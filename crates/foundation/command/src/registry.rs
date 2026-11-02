use std::collections::BTreeMap;

use crate::{Command, CommandError, Context, split_line};

/// A completion candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub help: String,
}

/// Every available command. Completion reads from here, so registering a
/// command also makes it a completion candidate.
#[derive(Default)]
pub struct CommandRegistry {
    commands: BTreeMap<String, Box<dyn Command>>,
}

impl CommandRegistry {
    pub fn register(&mut self, command: impl Command + 'static) {
        self.commands.insert(command.name().to_owned(), Box::new(command));
    }

    /// Commands whose names start with `prefix`, ignoring case, sorted by name.
    pub fn complete(&self, prefix: &str) -> Vec<Candidate> {
        let prefix = prefix.to_ascii_lowercase();
        self.commands
            .iter()
            .filter(|(name, _)| name.starts_with(&prefix))
            .map(|(name, command)| Candidate { name: name.clone(), help: command.help().to_owned() })
            .collect()
    }

    /// Parses and runs one command line. A blank line does nothing.
    pub fn execute(&self, line: &str, ctx: &mut Context<'_>) -> Result<(), CommandError> {
        let parts = split_line(line);
        let Some((name, args)) = parts.split_first() else {
            return Ok(());
        };
        let command = self
            .commands
            .get(&name.to_ascii_lowercase())
            .ok_or_else(|| CommandError::new(format!("Unknown command: {name}")))?;
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        command.run(ctx, &args)
    }
}

#[cfg(test)]
mod tests {
    use excel_edit::Document;

    use super::*;
    use crate::testing::{NoCalculator, NoStore};

    struct Echo(&'static str);

    impl Command for Echo {
        fn name(&self) -> &str {
            self.0
        }
        fn help(&self) -> &str {
            "Echo arguments"
        }
        fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
            ctx.say(args.join(","));
            Ok(())
        }
    }

    fn registry() -> CommandRegistry {
        let mut registry = CommandRegistry::default();
        registry.register(Echo("sort"));
        registry.register(Echo("sheet"));
        registry.register(Echo("find"));
        registry
    }

    fn names(candidates: Vec<Candidate>) -> Vec<String> {
        candidates.into_iter().map(|c| c.name).collect()
    }

    #[test]
    fn completion_filters_by_prefix() {
        let registry = registry();
        assert_eq!(names(registry.complete("s")), ["sheet", "sort"]);
        assert_eq!(names(registry.complete("SO")), ["sort"]);
        assert_eq!(registry.complete("").len(), 3);
        assert!(registry.complete("x").is_empty());
    }

    #[test]
    fn executes_with_arguments() {
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        registry().execute("SORT B desc", &mut ctx).unwrap();
        assert_eq!(ctx.message.as_deref(), Some("B,desc"));
    }

    #[test]
    fn unknown_commands_are_errors() {
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        let error = registry().execute("nope", &mut ctx).unwrap_err();
        assert_eq!(error.to_string(), "Unknown command: nope");
    }

    #[test]
    fn blank_lines_do_nothing() {
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        assert!(registry().execute("  ", &mut ctx).is_ok());
    }
}
