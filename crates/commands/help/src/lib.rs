//! Help command. The host decides how to show help: the GUI opens its help
//! panel, and the CLI prints the lists.

use excel_command::{Command, CommandError, CommandRegistry, Context, Effect};
use excel_model::Doc;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(Help);
}

struct Help;

impl Command for Help {
    fn name(&self) -> &str {
        "help"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Help",
            summary: "Show commands, functions, and keys",
            usage: "help [topic]",
            description: "Opens the help panel, also with F1. A topic narrows the list, and the exact name \
                          of a command or function opens its page.",
            args: &[("topic", "Optional. Text to search for, or a name")],
            examples: &[("help", ""), ("help sort", ""), ("help sum", "")],
            ..Doc::BLANK
        }
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
