use excel_command::{Command, CommandError, Confirmation, Context};
use excel_edit::Edit;
use excel_model::{CellInput, Doc, Sheet};

pub(crate) struct SheetCommand;

/// Excel's limit on sheet name length.
const MAX_NAME_LENGTH: usize = 31;

impl Command for SheetCommand {
    fn name(&self) -> &str {
        "sheet"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Sheet",
            summary: "List, switch, add, rename, or delete sheets",
            usage: "sheet [name | number | add <name> | rename <name> | delete]",
            description: "With nothing after it, lists the sheets. A name or number switches to that sheet, \
                          trying names first so a sheet named 2 can be reached. add creates a sheet. rename \
                          and delete act on the sheet showing: rename also updates formulas that refer to \
                          it, and delete asks first, turns references to the sheet into #REF!, and moves to \
                          the sheet before. sheet delete! deletes without asking. Undo brings either back.",
            args: &[
                ("name", "Optional. A sheet to switch to"),
                ("number", "Optional. A sheet's position, starting from 1"),
                ("add <name>", "Optional. Adds a sheet with this name"),
                ("rename <name>", "Optional. Gives the sheet showing a new name"),
                ("delete", "Optional. Deletes the sheet showing, after asking"),
            ],
            examples: &[
                ("sheet", ""),
                ("sheet 2", ""),
                ("sheet Data", ""),
                ("sheet add Summary", ""),
                ("sheet rename 매출 2026", ""),
                ("sheet delete", ""),
            ],
            ..Doc::BLANK
        }
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        match args {
            [] => list(ctx),
            ["add", words @ ..] if !words.is_empty() => add(ctx, &words.join(" "))?,
            ["add"] => return Err(CommandError::new("Usage: sheet add <name>")),
            ["rename", words @ ..] if !words.is_empty() => rename(ctx, &words.join(" "))?,
            ["rename"] => return Err(CommandError::new("Usage: sheet rename <new name>")),
            ["delete"] => return Err(ask_to_delete(ctx)),
            ["delete!"] => delete(ctx)?,
            ["delete" | "delete!", ..] => {
                return Err(CommandError::new("sheet delete acts on the sheet showing. Switch to it first"));
            }
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

/// Renames the sheet showing, and every reference to it in the workbook.
fn rename(ctx: &mut Context<'_>, name: &str) -> Result<(), CommandError> {
    validate_name(name)?;
    let active = ctx.doc.workbook.active_index();
    let old = ctx.doc.workbook.active().name.clone();
    if old == name {
        ctx.say(format!("The sheet is already named {name}"));
        return Ok(());
    }
    if ctx.doc.workbook.sheet_index(name).is_some_and(|index| index != active) {
        return Err(CommandError::new(format!("A sheet named {name} already exists")));
    }
    let calculator = ctx.calculator;
    let edit = Edit::replacing_sheets(&ctx.doc.workbook, |sheets| {
        sheets[active].name = name.to_owned();
        rewrite_formulas(sheets, |text| calculator.rename_sheet(text, &old, name));
    });
    ctx.apply(edit);
    ctx.say(format!("Renamed sheet {old} to {name}"));
    Ok(())
}

/// The question deleting the sheet showing asks first, or why it cannot be deleted.
fn ask_to_delete(ctx: &Context<'_>) -> CommandError {
    if ctx.doc.workbook.sheets().len() == 1 {
        return CommandError::new("The only sheet cannot be deleted");
    }
    let name = &ctx.doc.workbook.active().name;
    CommandError::confirm(
        format!("Deleting sheet {name} cannot be taken back except by undo. Use sheet delete! to go ahead"),
        Confirmation::Ask {
            question: format!("Delete sheet {name}?"),
            yes: "Delete".to_owned(),
            proceed: "sheet delete!".to_owned(),
        },
    )
}

/// Deletes the sheet showing and moves to the one before it.
fn delete(ctx: &mut Context<'_>) -> Result<(), CommandError> {
    if ctx.doc.workbook.sheets().len() == 1 {
        return Err(CommandError::new("The only sheet cannot be deleted"));
    }
    let active = ctx.doc.workbook.active_index();
    let name = ctx.doc.workbook.active().name.clone();
    let calculator = ctx.calculator;
    let edit = Edit::replacing_sheets(&ctx.doc.workbook, |sheets| {
        sheets.remove(active);
        rewrite_formulas(sheets, |text| calculator.remove_sheet(text, &name));
    });
    ctx.apply(edit);
    ctx.doc.workbook.set_active(active.saturating_sub(1));
    ctx.say(format!("Deleted sheet {name}"));
    Ok(())
}

fn rewrite_formulas(sheets: &mut [Sheet], rewrite: impl Fn(&str) -> String) {
    for sheet in sheets {
        for (_, cell) in sheet.iter_mut() {
            if let CellInput::Formula(text) = &cell.input {
                cell.input = CellInput::Formula(rewrite(text));
            }
        }
    }
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

    #[test]
    fn renames_the_sheet_showing() {
        let mut d = doc();
        run(&mut d, &["2"]).unwrap();
        assert_eq!(run(&mut d, &["rename", "매출", "2026"]).unwrap(), "Renamed sheet 요약 to 매출 2026");
        assert_eq!(d.workbook.active().name, "매출 2026");
        assert!(run(&mut d, &["rename", "data"]).is_err(), "names must stay unique, ignoring case");
        assert!(run(&mut d, &["rename", "a/b"]).is_err());
        assert!(run(&mut d, &["rename"]).is_err());
        d.undo();
        assert_eq!(d.workbook.sheets()[1].name, "요약");
    }

    #[test]
    fn deleting_asks_first_and_then_moves_to_the_sheet_before() {
        let mut d = doc();
        run(&mut d, &["2"]).unwrap();
        let error = run(&mut d, &["delete"]).unwrap_err();
        assert_eq!(error.confirmation.as_ref().map(Confirmation::proceed), Some("sheet delete!"));
        assert_eq!(d.workbook.sheets().len(), 3, "nothing is deleted before an answer");

        assert_eq!(run(&mut d, &["delete!"]).unwrap(), "Deleted sheet 요약");
        let names: Vec<&str> = d.workbook.sheets().iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Data", "3"]);
        assert_eq!(d.workbook.active().name, "Data");
        d.undo();
        assert_eq!(d.workbook.sheets().len(), 3);
        assert!(run(&mut d, &["delete", "Data"]).is_err(), "only the sheet showing");
    }

    #[test]
    fn the_only_sheet_stays() {
        let mut d = Document::default();
        let error = run(&mut d, &["delete"]).unwrap_err();
        assert!(error.confirmation.is_none());
        assert!(run(&mut d, &["delete!"]).is_err());
        assert_eq!(d.workbook.sheets().len(), 1);
    }
}
