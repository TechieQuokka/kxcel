use excel_command::{Command, CommandError, Context, Structural};

use crate::target::{Target, apply};

pub(crate) struct Delete;

impl Command for Delete {
    fn name(&self) -> &str {
        "delete"
    }

    fn help(&self) -> &str {
        "Delete rows or columns: delete row 3 2, delete col C"
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        let target = Target::parse(ctx, args, "Usage: delete <row|col> [position] [count]")?;
        let Target { at, count, .. } = target;
        let change =
            if target.rows { Structural::DeleteRows { at, count } } else { Structural::DeleteCols { at, count } };
        apply(ctx, change);
        ctx.say(format!("Deleted {}", target.describe()));
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
        for addr in ["A1", "A2", "A3", "B1"] {
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
        Delete.run(&mut ctx, args)?;
        Ok(ctx.message.unwrap())
    }

    #[test]
    fn deletes_the_cursor_row_by_default() {
        let mut d = doc();
        assert_eq!(run(&mut d, &["row"]).unwrap(), "Deleted 1 row at 2");
        assert_eq!(cells(&d), ["A1=A1", "B1=B1", "A2=A3"]);
    }

    #[test]
    fn deletes_columns() {
        let mut d = doc();
        run(&mut d, &["col", "A"]).unwrap();
        assert_eq!(cells(&d), ["A1=B1"]);
    }

    #[test]
    fn a_large_count_stops_at_the_sheet_edge() {
        let mut d = doc();
        assert_eq!(run(&mut d, &["col", "XFD", "5"]).unwrap(), "Deleted 1 column at XFD");
    }

    #[test]
    fn can_be_undone() {
        let mut d = doc();
        run(&mut d, &["row", "1", "2"]).unwrap();
        d.undo();
        assert_eq!(d.workbook.active().len(), 4);
    }
}
