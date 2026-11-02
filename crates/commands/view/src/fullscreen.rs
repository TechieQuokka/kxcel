use excel_command::{Command, CommandError, Context, Effect};
use excel_model::Doc;

use crate::switch::on_off;

pub(crate) struct Fullscreen;

impl Command for Fullscreen {
    fn name(&self) -> &str {
        "fullscreen"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "View",
            summary: "Full screen on or off",
            usage: "fullscreen [on|off]",
            description: "Fills the screen with the window, or returns it to normal. Without on or off, \
                          switches between them, as F11 does. Only the app window has a full screen.",
            args: &[("on|off", "Optional. Which to turn to. Switches when left out")],
            examples: &[("fullscreen", ""), ("fullscreen off", "")],
            related: &["theme"],
        }
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        let on = on_off(args, "fullscreen [on|off]")?;
        ctx.effects.push(Effect::Fullscreen(on));
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
        Fullscreen.run(&mut ctx, args)?;
        Ok(ctx.effects)
    }

    #[test]
    fn asks_the_host() {
        assert_eq!(effects(&[]), Ok(vec![Effect::Fullscreen(None)]));
        assert_eq!(effects(&["on"]), Ok(vec![Effect::Fullscreen(Some(true))]));
        assert_eq!(effects(&["off"]), Ok(vec![Effect::Fullscreen(Some(false))]));
        assert!(effects(&["big"]).is_err());
    }
}
