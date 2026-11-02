use std::collections::BTreeMap;

use excel_model::Doc;

use crate::{Command, CommandError, Context, split_line};

/// A completion candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    /// How to write it, such as `sort <column> [desc] [header]`.
    pub usage: String,
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
            .map(|(name, command)| {
                let doc = command.doc();
                Candidate { name: name.clone(), usage: doc.usage.to_owned(), help: doc.summary.to_owned() }
            })
            .collect()
    }

    /// Every command's name and help, sorted by name.
    pub fn docs(&self) -> Vec<(String, Doc)> {
        self.commands.iter().map(|(name, command)| (name.clone(), command.doc())).collect()
    }

    /// Whether the named command takes a file path, ignoring case.
    pub fn takes_path(&self, name: &str) -> bool {
        self.commands.get(&name.to_ascii_lowercase()).is_some_and(|command| command.takes_path())
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
        if command.takes_raw_line() {
            let rest = line.trim_start().split_once(char::is_whitespace).map_or("", |(_, rest)| rest.trim_start());
            let args: Vec<&str> = if rest.is_empty() { Vec::new() } else { vec![rest] };
            return command.run(ctx, &args);
        }
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
        fn doc(&self) -> Doc {
            Doc { summary: "Echo arguments", usage: "echo", ..Doc::BLANK }
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

    struct Raw;

    impl Command for Raw {
        fn name(&self) -> &str {
            "raw"
        }
        fn doc(&self) -> Doc {
            Doc { summary: "Echo the line", usage: "raw", ..Doc::BLANK }
        }
        fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
            ctx.say(format!("{args:?}"));
            Ok(())
        }
        fn takes_raw_line(&self) -> bool {
            true
        }
    }

    #[test]
    fn a_raw_line_keeps_quotes_and_spaces() {
        let mut registry = registry();
        registry.register(Raw);
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        registry.execute(r#"  raw  A1 =IF(B1="예", "네  개", 0)"#, &mut ctx).unwrap();
        assert_eq!(ctx.message.as_deref(), Some(r#"["A1 =IF(B1=\"예\", \"네  개\", 0)"]"#));
        registry.execute("raw", &mut ctx).unwrap();
        assert_eq!(ctx.message.as_deref(), Some("[]"));
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
