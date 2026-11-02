use excel_command::{Command, CommandError, Context, Structural};
use excel_model::Doc;

use crate::target::{Target, apply};

pub(crate) struct Insert;

impl Command for Insert {
    fn name(&self) -> &str {
        "insert"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Edit",
            summary: "Insert rows or columns",
            usage: "insert <row|col> [position] [count]",
            description: "Inserts empty rows or columns and shifts the rest down or right. Formulas are \
                          adjusted to keep pointing at the same cells.",
            args: &[
                ("row|col", "Whether to insert rows or columns"),
                ("position", "Optional. A row number or column letter. The cursor's by default"),
                ("count", "Optional. How many. 1 by default"),
            ],
            examples: &[("insert row 3 2", ""), ("insert col C", "")],
            related: &["delete"],
        }
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        let target = Target::parse(ctx, args, "Usage: insert <row|col> [position] [count]")?;
        let Target { at, count, .. } = target;
        let change =
            if target.rows { Structural::InsertRows { at, count } } else { Structural::InsertCols { at, count } };
        apply(ctx, change);
        ctx.say(format!("Inserted {}", target.describe()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{NoCalculator, NoStore};
    use excel_edit::Document;
    use excel_model::{Cell, CellAddr};

    use super::*;

    fn doc() -> Document {
        let mut doc = Document::default();
        for addr in ["A1", "A2", "B1"] {
            doc.workbook.active_mut().set(addr.parse().unwrap(), Cell::parse_input(addr));
        }
        doc
    }

    fn cells(doc: &Document) -> Vec<String> {
        doc.workbook.active().iter().map(|(addr, cell)| format!("{addr}={}", cell.value)).collect()
    }

    fn run(doc: &mut Document, args: &[&str]) -> Result<String, CommandError> {
        let mut ctx = Context::new(doc, &NoCalculator, &NoStore);
        ctx.cursor = CellAddr::new(1, 0);
        Insert.run(&mut ctx, args)?;
        Ok(ctx.message.unwrap())
    }

    #[test]
    fn inserts_rows_at_the_cursor_by_default() {
        let mut d = doc();
        assert_eq!(run(&mut d, &["row"]).unwrap(), "Inserted 1 row at 2");
        assert_eq!(cells(&d), ["A1=A1", "B1=B1", "A3=A2"]);
    }

    #[test]
    fn inserts_columns_at_a_position() {
        let mut d = doc();
        assert_eq!(run(&mut d, &["col", "A", "2"]).unwrap(), "Inserted 2 columns at A");
        assert_eq!(cells(&d), ["C1=A1", "D1=B1", "C2=A2"]);
    }

    #[test]
    fn can_be_undone() {
        let mut d = doc();
        run(&mut d, &["row", "1"]).unwrap();
        d.undo();
        assert_eq!(cells(&d), ["A1=A1", "B1=B1", "A2=A2"]);
    }

    #[test]
    fn rejects_bad_arguments() {
        let mut d = doc();
        for args in [
            &[][..],
            &["cell"],
            &["row", "0"],
            &["row", "x"],
            &["col", "1"],
            &["row", "1", "0"],
            &["row", "1", "1", "1"],
        ] {
            assert!(run(&mut d, args).is_err(), "{args:?}");
        }
    }
}
