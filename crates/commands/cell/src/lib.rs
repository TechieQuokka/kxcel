//! Command that sets a cell, so the CLI can enter data without a window.

use excel_command::{Command, CommandError, CommandRegistry, Context};
use excel_edit::{CellChange, Edit};
use excel_model::Doc;
use excel_model::{Cell, CellAddr};

pub fn register(registry: &mut CommandRegistry) {
    registry.register(Set);
}

struct Set;

impl Command for Set {
    fn name(&self) -> &str {
        "set"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Edit",
            summary: "Set a cell's value or formula",
            usage: "set <cell> <value>",
            description: "Writes to a cell as if it were typed there, so a value starting with = is a formula. \
                          Mostly useful in kxcel-cli, which has no grid to type into.",
            args: &[("cell", "The cell, such as B5"), ("value", "A number, text, or =formula. The rest of the line")],
            examples: &[("set B5 =A1*2", ""), ("set A1 사과", "")],
            related: &["goto"],
        }
    }

    fn takes_raw_line(&self) -> bool {
        true
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        let line = args.join(" ");
        let (address, value) = line.split_once(char::is_whitespace).unwrap_or((&line, ""));
        if address.is_empty() {
            return Err(CommandError::new("Usage: set <cell> <value>"));
        }
        let addr: CellAddr = address.parse().map_err(CommandError::new)?;
        let sheet = ctx.doc.workbook.active_index();
        let before = ctx.doc.workbook.active().get(addr).cloned();
        let after = Cell::parse_input(value.trim_start());
        ctx.apply(Edit::SetCells(vec![CellChange { sheet, addr, before, after }]));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{NoCalculator, NoStore};
    use excel_edit::Document;
    use excel_model::CellValue;

    use super::*;

    fn run(doc: &mut Document, args: &[&str]) -> Result<(), CommandError> {
        let mut ctx = Context::new(doc, &NoCalculator, &NoStore);
        Set.run(&mut ctx, args)
    }

    #[test]
    fn sets_and_clears_a_cell() {
        let mut doc = Document::default();
        let a1 = "A1".parse().unwrap();
        run(&mut doc, &["A1", "5"]).unwrap();
        assert_eq!(doc.workbook.active().value(a1), &CellValue::Number(5.0));
        run(&mut doc, &["A1"]).unwrap();
        assert!(doc.workbook.active().get(a1).is_none());
    }

    #[test]
    fn joins_words_into_text() {
        let mut doc = Document::default();
        run(&mut doc, &["A1", "사과", "두개"]).unwrap();
        assert_eq!(doc.workbook.active().value("A1".parse().unwrap()), &CellValue::Text("사과 두개".into()));
    }

    #[test]
    fn can_be_undone() {
        let mut doc = Document::default();
        run(&mut doc, &["A1", "5"]).unwrap();
        doc.undo();
        assert!(doc.workbook.active().is_empty());
    }

    #[test]
    fn rejects_bad_arguments() {
        let mut doc = Document::default();
        assert!(run(&mut doc, &[]).is_err());
        assert!(run(&mut doc, &["nope", "1"]).is_err());
    }
}
