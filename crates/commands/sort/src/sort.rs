use excel_command::{Command, CommandError, Context};
use excel_edit::Edit;
use excel_model::Doc;
use excel_model::{CellRange, col_from_name};

pub(crate) struct Sort;

const USAGE: &str = "Usage: sort <column> [desc] [header]";

impl Command for Sort {
    fn name(&self) -> &str {
        "sort"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Edit",
            summary: "Sort rows by a column",
            usage: "sort <column> [desc] [header]",
            description: "Sorts the selection if it spans several cells, otherwise every used row. Numbers \
                          come before text, then booleans and errors, and empty cells always go last. \
                          Formulas move with their rows.",
            args: &[
                ("column", "The column to sort by, such as B"),
                ("desc", "Optional. Largest first"),
                ("header", "Optional. Keeps the first row in place"),
            ],
            examples: &[("sort B", ""), ("sort C desc header", "")],
            ..Doc::BLANK
        }
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        let [column, options @ ..] = args else {
            return Err(CommandError::new(USAGE));
        };
        let col = col_from_name(column).ok_or_else(|| CommandError::new(format!("Invalid column: {column}")))?;
        let (mut descending, mut header) = (false, false);
        for option in options {
            match option.to_ascii_lowercase().as_str() {
                "desc" => descending = true,
                "asc" => descending = false,
                "header" => header = true,
                _ => return Err(CommandError::new(USAGE)),
            }
        }

        let range = target_range(ctx);
        let Some(mut range) = range else {
            return Err(CommandError::new("Nothing to sort"));
        };
        if !(range.start.col..=range.end.col).contains(&col) {
            return Err(CommandError::new(format!("Column {column} is outside {range}")));
        }
        if header {
            if range.rows() < 2 {
                return Err(CommandError::new("Nothing to sort below the header"));
            }
            range.start.row += 1;
        }

        let active = ctx.doc.workbook.active_index();
        let calculator = ctx.calculator;
        let edit = Edit::replacing_sheets(&ctx.doc.workbook, |sheets| {
            sheets[active].sort_rows(range, col, descending, |text, rows| calculator.translate(text, rows, 0));
        });
        ctx.apply(edit);
        ctx.say(format!("Sorted {range} by {}", column.to_ascii_uppercase()));
        Ok(())
    }
}

/// Sorts the selection if it spans several cells, otherwise the sheet's used cells.
fn target_range(ctx: &Context<'_>) -> Option<CellRange> {
    if ctx.selection.is_single() { ctx.doc.workbook.active().used_range() } else { Some(ctx.selection) }
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{NoCalculator, NoStore};
    use excel_edit::Document;
    use excel_model::Cell;

    use super::*;

    fn doc(cells: &[(&str, &str)]) -> Document {
        let mut doc = Document::default();
        for (addr, text) in cells {
            doc.workbook.active_mut().set(addr.parse().unwrap(), Cell::parse_input(text));
        }
        doc
    }

    fn run(doc: &mut Document, selection: Option<&str>, args: &[&str]) -> Result<(), CommandError> {
        let mut ctx = Context::new(doc, &NoCalculator, &NoStore);
        if let Some(range) = selection {
            ctx.selection = range.parse().unwrap();
        }
        Sort.run(&mut ctx, args)
    }

    fn column(doc: &Document, col: &str, rows: std::ops::RangeInclusive<u32>) -> Vec<String> {
        rows.map(|row| doc.workbook.active().value(format!("{col}{row}").parse().unwrap()).to_string()).collect()
    }

    #[test]
    fn sorts_whole_rows_of_the_used_range() {
        let mut d = doc(&[("A1", "c"), ("B1", "3"), ("A2", "a"), ("B2", "1"), ("A3", "b"), ("B3", "2")]);
        run(&mut d, None, &["B"]).unwrap();
        assert_eq!(column(&d, "A", 1..=3), ["a", "b", "c"]);
        assert_eq!(column(&d, "B", 1..=3), ["1", "2", "3"]);
    }

    #[test]
    fn sorts_descending_and_keeps_a_header() {
        let mut d = doc(&[("A1", "Name"), ("A2", "1"), ("A3", "3"), ("A4", "2")]);
        run(&mut d, None, &["a", "desc", "header"]).unwrap();
        assert_eq!(column(&d, "A", 1..=4), ["Name", "3", "2", "1"]);
    }

    #[test]
    fn sorts_only_the_selection() {
        let mut d = doc(&[("A1", "3"), ("A2", "2"), ("A3", "1"), ("B1", "x")]);
        run(&mut d, Some("A2:A3"), &["A"]).unwrap();
        assert_eq!(column(&d, "A", 1..=3), ["3", "1", "2"]);
    }

    #[test]
    fn mixed_types_order_with_empties_last() {
        let mut d = doc(&[("A1", "z"), ("A2", "5"), ("A3", "TRUE"), ("A5", "1"), ("B4", "keep")]);
        run(&mut d, None, &["A"]).unwrap();
        assert_eq!(column(&d, "A", 1..=5), ["1", "5", "z", "TRUE", ""]);
        run(&mut d, None, &["A", "desc"]).unwrap();
        assert_eq!(column(&d, "A", 1..=5), ["TRUE", "z", "5", "1", ""]);
    }

    #[test]
    fn equal_keys_keep_their_order() {
        let mut d = doc(&[("A1", "1"), ("B1", "first"), ("A2", "1"), ("B2", "second"), ("A3", "0"), ("B3", "zero")]);
        run(&mut d, None, &["A"]).unwrap();
        assert_eq!(column(&d, "B", 1..=3), ["zero", "first", "second"]);
    }

    #[test]
    fn can_be_undone() {
        let mut d = doc(&[("A1", "2"), ("A2", "1")]);
        run(&mut d, None, &["A"]).unwrap();
        d.undo();
        assert_eq!(column(&d, "A", 1..=2), ["2", "1"]);
    }

    #[test]
    fn rejects_bad_arguments() {
        let mut d = doc(&[("A1", "1"), ("A2", "2")]);
        assert!(run(&mut d, None, &[]).is_err());
        assert!(run(&mut d, None, &["1"]).is_err());
        assert!(run(&mut d, None, &["C"]).is_err(), "outside the range");
        assert!(run(&mut d, None, &["A", "sideways"]).is_err());
        let mut empty = Document::default();
        assert!(run(&mut empty, None, &["A"]).is_err());
    }
}
