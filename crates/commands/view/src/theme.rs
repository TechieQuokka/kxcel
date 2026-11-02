use excel_command::{Command, CommandError, Context, Effect, Theme};
use excel_model::Doc;

pub(crate) struct ThemeCommand;

impl Command for ThemeCommand {
    fn name(&self) -> &str {
        "theme"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "View",
            summary: "Dark or light colors",
            usage: "theme [dark|light]",
            description: "Changes the colors of the window. Without dark or light, switches to the other. \
                          The choice is remembered for the next start.",
            args: &[("dark|light", "Optional. Which theme to use. Switches when left out")],
            examples: &[("theme light", ""), ("theme", "")],
            related: &["fullscreen"],
        }
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        let theme = match args {
            [] => None,
            [name] => Some(Theme::from_name(name).ok_or_else(|| CommandError::new("Usage: theme [dark|light]"))?),
            _ => return Err(CommandError::new("Usage: theme [dark|light]")),
        };
        ctx.effects.push(Effect::Theme(theme));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{NoCalculator, NoStore};
    use excel_edit::Document;

    use super::*;

    fn effects(args: &[&str]) -> Result<Vec<Effect>, CommandError> {
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        ThemeCommand.run(&mut ctx, args)?;
        Ok(ctx.effects)
    }

    #[test]
    fn asks_the_host_for_a_theme() {
        assert_eq!(effects(&[]), Ok(vec![Effect::Theme(None)]));
        assert_eq!(effects(&["Light"]), Ok(vec![Effect::Theme(Some(Theme::Light))]));
        assert_eq!(effects(&["dark"]), Ok(vec![Effect::Theme(Some(Theme::Dark))]));
        assert!(effects(&["blue"]).is_err());
    }
}
