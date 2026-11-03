use excel_command::{Command, CommandError, Context, Effect};
use excel_model::CellAddr;
use excel_model::Doc;

pub(crate) struct Goto;

impl Command for Goto {
    fn name(&self) -> &str {
        "goto"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Navigate",
            summary: "Move to a cell",
            usage: "goto <cell>",
            args: &[("cell", "The cell, such as B5")],
            examples: &[("goto B5", ""), ("goto A100000", "")],
            related: &["find", "sheet"],
            ..Doc::BLANK
        }
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        let [address] = args else {
            return Err(CommandError::new("Usage: goto <cell>"));
        };
        let addr: CellAddr = address.parse().map_err(CommandError::new)?;
        ctx.cursor = addr;
        ctx.effects.push(Effect::MoveCursor(addr));
        if ctx.doc.workbook.active().is_row_hidden(addr.row) {
            ctx.say(format!(
                "Row {} is hidden by a filter, so the cursor is on the nearest row that shows",
                addr.row + 1
            ));
        }
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

    #[test]
    fn says_when_the_row_is_hidden() {
        let mut doc = Document::default();
        let mut meta = doc.workbook.active().meta().clone();
        meta.hidden_rows.insert(4);
        doc.workbook.active_mut().set_meta(meta);
        let mut ctx = Context::new(&mut doc, &NoCalculator, &NoStore);
        Goto.run(&mut ctx, &["b5"]).unwrap();
        assert!(ctx.message.unwrap().contains("Row 5 is hidden"));
    }
}
