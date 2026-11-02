use excel_command::{Command, CommandError, Context, Effect};
use excel_model::Doc;
use excel_model::{Cell, CellAddr};

pub(crate) struct Find;

impl Command for Find {
    fn name(&self) -> &str {
        "find"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Navigate",
            summary: "Move to the next cell containing text",
            usage: "find <text>",
            description: "Searches shown values and formula text, ignoring case, from just after the cursor \
                          and wrapping around. Running it again moves to the next match.",
            args: &[("text", "What to look for. The rest of the line")],
            examples: &[("find apple", ""), ("find =SUM", "")],
            related: &["goto"],
        }
    }

    /// Searches shown values and formula text, ignoring case, from just after
    /// the cursor and wrapping around. Running it again moves to the next match.
    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        if args.is_empty() {
            return Err(CommandError::new("Usage: find <text>"));
        }
        let needle = args.join(" ").to_lowercase();
        let matches: Vec<CellAddr> =
            ctx.doc.workbook.active().iter().filter(|(_, cell)| matches(cell, &needle)).map(|(addr, _)| addr).collect();
        if matches.is_empty() {
            return Err(CommandError::new(format!("Not found: {}", args.join(" "))));
        }

        let next = matches.iter().position(|addr| *addr > ctx.cursor).unwrap_or(0);
        let found = matches[next];
        ctx.effects.push(Effect::MoveCursor(found));
        ctx.cursor = found;
        ctx.say(format!("{found} ({} of {})", next + 1, matches.len()));
        Ok(())
    }
}

fn matches(cell: &Cell, needle: &str) -> bool {
    cell.display().to_lowercase().contains(needle) || cell.input_text().to_lowercase().contains(needle)
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{NoCalculator, NoStore};
    use excel_edit::Document;

    use super::*;

    fn doc() -> Document {
        let mut doc = Document::default();
        for (addr, text) in [("A1", "사과"), ("C1", "x"), ("B2", "Apple"), ("A3", "apple pie"), ("D4", "=SUM(A1:A2)")]
        {
            doc.workbook.active_mut().set(addr.parse().unwrap(), Cell::parse_input(text));
        }
        doc
    }

    fn find_from(doc: &mut Document, cursor: &str, args: &[&str]) -> Result<(CellAddr, String), CommandError> {
        let mut ctx = Context::new(doc, &NoCalculator, &NoStore);
        ctx.cursor = cursor.parse().unwrap();
        Find.run(&mut ctx, args)?;
        Ok((ctx.cursor, ctx.message.unwrap()))
    }

    #[test]
    fn moves_to_the_next_match_ignoring_case() {
        let mut d = doc();
        let (found, message) = find_from(&mut d, "A1", &["APPLE"]).unwrap();
        assert_eq!(found.to_string(), "B2");
        assert_eq!(message, "B2 (1 of 2)");
        assert_eq!(find_from(&mut d, "B2", &["apple"]).unwrap().0.to_string(), "A3");
    }

    #[test]
    fn wraps_around() {
        let mut d = doc();
        assert_eq!(find_from(&mut d, "A3", &["apple"]).unwrap().0.to_string(), "B2");
    }

    #[test]
    fn finds_korean_and_multiple_words() {
        let mut d = doc();
        assert_eq!(find_from(&mut d, "A5", &["사과"]).unwrap().0.to_string(), "A1");
        assert_eq!(find_from(&mut d, "A1", &["apple", "pie"]).unwrap().0.to_string(), "A3");
    }

    #[test]
    fn searches_formula_text() {
        let mut d = doc();
        assert_eq!(find_from(&mut d, "A1", &["sum("]).unwrap().0.to_string(), "D4");
    }

    #[test]
    fn reports_when_nothing_matches() {
        let mut d = doc();
        assert!(find_from(&mut d, "A1", &["zzz"]).is_err());
        assert!(find_from(&mut d, "A1", &[]).is_err());
    }
}
