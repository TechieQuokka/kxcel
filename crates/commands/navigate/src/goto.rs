use excel_command::{Command, CommandError, Context, Effect};
use excel_model::CellAddr;

pub(crate) struct Goto;

impl Command for Goto {
    fn name(&self) -> &str {
        "goto"
    }

    fn help(&self) -> &str {
        "Move to a cell: goto B5"
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        let [address] = args else {
            return Err(CommandError::new("Usage: goto <cell>"));
        };
        let addr: CellAddr = address.parse().map_err(CommandError::new)?;
        ctx.cursor = addr;
        ctx.effects.push(Effect::MoveCursor(addr));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{NoCalculator, NoStore};
    use excel_edit::Document;

    use super::*;

    #[test]
    fn requests_a_cursor_move() {
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        Goto.run(&mut ctx, &["b5"]).unwrap();
        assert_eq!(ctx.effects, [Effect::MoveCursor(CellAddr::new(4, 1))]);
    }

    #[test]
    fn rejects_bad_addresses() {
        let mut doc = Document::default();
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        assert!(Goto.run(&mut ctx, &[]).is_err());
        assert!(Goto.run(&mut ctx, &["5B"]).is_err());
        assert!(Goto.run(&mut ctx, &["A0"]).is_err());
    }
}
