use excel_command::{Command, CommandError, Context};
use excel_edit::Edit;
use excel_model::Sheet;

pub(crate) struct SheetCommand;

/// Excel's limit on sheet name length.
const MAX_NAME_LENGTH: usize = 31;

impl Command for SheetCommand {
    fn name(&self) -> &str {
        "sheet"
    }

    fn help(&self) -> &str {
        "List, switch, or add sheets: sheet 2, sheet Data, sheet add Name"
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        match args {
            [] => list(ctx),
            ["add", words @ ..] if !words.is_empty() => add(ctx, &words.join(" "))?,
            ["add"] => return Err(CommandError::new("Usage: sheet add <name>")),
            words => switch(ctx, &words.join(" "))?,
        }
        Ok(())
    }
}

fn list(ctx: &mut Context<'_>) {
    let workbook = &ctx.doc.workbook;
    let names: Vec<String> = workbook
        .sheets()
        .iter()
        .enumerate()
        .map(|(index, sheet)| {
            let mark = if index == workbook.active_index() { "*" } else { "" };
            format!("{}:{}{mark}", index + 1, sheet.name)
        })
        .collect();
    ctx.say(names.join("  "));
}

/// Switches by name first, so a sheet named `2` can be reached, then by position.
fn switch(ctx: &mut Context<'_>, target: &str) -> Result<(), CommandError> {
    let workbook = &ctx.doc.workbook;
    let by_position =
        || target.parse::<usize>().ok().and_then(|n| n.checked_sub(1)).filter(|i| *i < workbook.sheets().len());
    let index = workbook
        .sheet_index(target)
        .or_else(by_position)
        .ok_or_else(|| CommandError::new(format!("No such sheet: {target}")))?;
    ctx.doc.workbook.set_active(index);
    let name = ctx.doc.workbook.active().name.clone();
    ctx.say(format!("Sheet {name}"));
    Ok(())
}

fn add(ctx: &mut Context<'_>, name: &str) -> Result<(), CommandError> {
    validate_name(name)?;
    if ctx.doc.workbook.sheet_index(name).is_some() {
        return Err(CommandError::new(format!("A sheet named {name} already exists")));
    }
    let edit = Edit::replacing_sheets(&ctx.doc.workbook, |sheets| sheets.push(Sheet::new(name)));
    ctx.apply(edit);
    let last = ctx.doc.workbook.sheets().len() - 1;
    ctx.doc.workbook.set_active(last);
    ctx.say(format!("Added sheet {name}"));
    Ok(())
}

/// Applies the rules `.xlsx` files have for sheet names.
fn validate_name(name: &str) -> Result<(), CommandError> {
    if name.chars().count() > MAX_NAME_LENGTH {
        return Err(CommandError::new(format!("Sheet names are limited to {MAX_NAME_LENGTH} characters")));
    }
    if name.contains(['[', ']', ':', '*', '?', '/', '\\']) {
        return Err(CommandError::new("Sheet names cannot contain [ ] : * ? / \\"));
    }
    if name.starts_with('\'') || name.ends_with('\'') {
        return Err(CommandError::new("Sheet names cannot start or end with '"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{NoCalculator, NoStore};
    use excel_edit::Document;
    use excel_model::Workbook;

    use super::*;

    fn doc() -> Document {
        let book = Workbook::from_sheets(vec![Sheet::new("Data"), Sheet::new("요약"), Sheet::new("3")]);
        Document::new(book, None, Vec::new())
    }

    fn run(doc: &mut Document, args: &[&str]) -> Result<String, CommandError> {
        let mut ctx = Context::new(doc, &NoCalculator, &NoStore);
        SheetCommand.run(&mut ctx, args)?;
        Ok(ctx.message.unwrap_or_default())
    }

    #[test]
    fn lists_sheets_and_marks_the_active_one() {
        let mut d = doc();
        assert_eq!(run(&mut d, &[]).unwrap(), "1:Data*  2:요약  3:3");
    }

    #[test]
    fn switches_by_name_or_position() {
        let mut d = doc();
        run(&mut d, &["요약"]).unwrap();
        assert_eq!(d.workbook.active_index(), 1);
        run(&mut d, &["1"]).unwrap();
        assert_eq!(d.workbook.active_index(), 0);
        run(&mut d, &["3"]).unwrap();
        assert_eq!(d.workbook.active().name, "3");
    }

    #[test]
    fn switching_does_not_mark_changes() {
        let mut d = doc();
        run(&mut d, &["2"]).unwrap();
        assert!(!d.dirty);
    }

    #[test]
    fn rejects_unknown_sheets() {
        let mut d = doc();
        assert!(run(&mut d, &["Nope"]).is_err());
        assert!(run(&mut d, &["0"]).is_err());
        assert!(run(&mut d, &["9"]).is_err());
    }

    #[test]
    fn adds_a_sheet_and_switches_to_it() {
        let mut d = doc();
        run(&mut d, &["add", "New", "One"]).unwrap();
        assert_eq!(d.workbook.active().name, "New One");
        d.undo();
        assert_eq!(d.workbook.sheets().len(), 3);
        assert_eq!(d.workbook.active().name, "3");
    }

    #[test]
    fn rejects_invalid_new_names() {
        let mut d = doc();
        for args in [&["add"][..], &["add", "data"], &["add", "a/b"], &["add", "'x'"]] {
            assert!(run(&mut d, args).is_err(), "{args:?}");
        }
        let long = "x".repeat(32);
        assert!(run(&mut d, &["add", &long]).is_err());
    }
}
