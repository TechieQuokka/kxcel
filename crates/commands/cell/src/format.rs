//! The `format` command: whether numbers show with commas.

use excel_command::{Command, CommandError, Context};
use excel_edit::Edit;
use excel_model::{CellAddr, CellRange, Doc, MAX_ROWS, col_from_name};

pub(crate) struct Format;

const USAGE: &str = "Usage: format <columns or range> <plain|comma>";

impl Command for Format {
    fn name(&self) -> &str {
        "format"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Edit",
            summary: "Show numbers with or without commas",
            usage: "format <columns or range> <plain|comma>",
            description: "Numbers show with commas between groups of three digits, such as 1,234,567. plain \
                          shows them without, for numbers such as years or codes, and comma goes back. A \
                          column such as A, or columns such as A:C, cover every row, empty cells too, so \
                          numbers typed there later follow it. Dates are never affected.",
            args: &[
                ("columns or range", "A column such as A, columns such as A:C, or a range such as B2:B10"),
                ("plain|comma", "plain shows numbers without commas, comma with them"),
            ],
            examples: &[("format A plain", ""), ("format B2:D9 plain", ""), ("format A comma", "")],
            ..Doc::BLANK
        }
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        let [target, style] = args else {
            return Err(CommandError::new(USAGE));
        };
        let range =
            parse_target(target).ok_or_else(|| CommandError::new(format!("Invalid columns or range: {target}")))?;
        let plain = match style.to_ascii_lowercase().as_str() {
            "plain" => true,
            "comma" => false,
            _ => return Err(CommandError::new(USAGE)),
        };
        let index = ctx.doc.workbook.active_index();
        let before = ctx.doc.workbook.active().meta().clone();
        let mut after = before.clone();
        if plain {
            after.plain_cells.add(range);
        } else {
            after.plain_cells.remove(range);
        }
        if after != before {
            ctx.apply(Edit::SetMeta { sheet: index, before, after });
        }
        let how = if plain { "without commas" } else { "with commas" };
        ctx.say(format!("Numbers in {target} show {how}", target = target.to_ascii_uppercase()));
        Ok(())
    }
}

/// Reads `A`, `A:C`, `B2`, or `B2:D9`. Columns cover every row.
fn parse_target(text: &str) -> Option<CellRange> {
    if text.chars().any(|c| c.is_ascii_digit()) {
        return text.parse().ok();
    }
    let (first, last) = text.split_once(':').unwrap_or((text, text));
    let (first, last) = (col_from_name(first)?, col_from_name(last)?);
    Some(CellRange::new(CellAddr::new(0, first.min(last)), CellAddr::new(MAX_ROWS - 1, first.max(last))))
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{NoCalculator, NoStore};
    use excel_edit::Document;
    use excel_model::Cell;

    use super::*;

    fn run(doc: &mut Document, args: &[&str]) -> Result<String, CommandError> {
        let mut ctx = Context::new(doc, &NoCalculator, &NoStore);
        Format.run(&mut ctx, args)?;
        Ok(ctx.message.unwrap_or_default())
    }

    #[test]
    fn plain_and_back_with_undo() {
        let mut d = Document::default();
        d.workbook.active_mut().set("A5".parse().unwrap(), Cell::parse_input("2026"));
        let shown = |d: &Document| d.workbook.active().shown("A5".parse().unwrap());
        assert_eq!(shown(&d), "2,026");
        assert_eq!(run(&mut d, &["a", "plain"]).unwrap(), "Numbers in A show without commas");
        assert_eq!(shown(&d), "2026");
        run(&mut d, &["A5", "comma"]).unwrap();
        assert_eq!(shown(&d), "2,026");
        d.undo();
        assert_eq!(shown(&d), "2026");
        d.undo();
        assert_eq!(shown(&d), "2,026");
    }

    #[test]
    fn reads_columns_and_ranges() {
        assert_eq!(parse_target("C:A").unwrap().to_string(), "A1:C1048576");
        assert_eq!(parse_target("B2:D9").unwrap().to_string(), "B2:D9");
        assert!(parse_target("1:2").is_none());
        let mut d = Document::default();
        for args in [&[][..], &["A"], &["A", "bold"], &["?", "plain"]] {
            assert!(run(&mut d, args).is_err(), "{args:?}");
        }
    }
}
