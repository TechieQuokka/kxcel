//! Help command. The host decides how to show help: the GUI opens its help
//! panel, and the CLI prints the lists.

use excel_command::{Command, CommandError, CommandRegistry, Context, Effect};

pub fn register(registry: &mut CommandRegistry) {
    registry.register(Help);
}

struct Help;

impl Command for Help {
    fn name(&self) -> &str {
        "help"
    }

    fn help(&self) -> &str {
        "Show commands, functions, and keys: help, help sort"
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        let topic = (!args.is_empty()).then(|| args.join(" "));
        ctx.effects.push(Effect::ShowHelp(topic));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{NoCalculator, NoStore};
    use excel_edit::Document;

    use super::*;

    fn effects(args: &[&str]) -> Vec<Effect> {
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        Help.run(&mut ctx, args).unwrap();
        ctx.effects
    }

    #[test]
    fn asks_the_host_to_show_help() {
        assert_eq!(effects(&[]), [Effect::ShowHelp(None)]);
        assert_eq!(effects(&["sort"]), [Effect::ShowHelp(Some("sort".into()))]);
    }
}
