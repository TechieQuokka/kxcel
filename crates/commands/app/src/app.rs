use excel_command::{Command, CommandError, Context};
use excel_edit::Edit;
use excel_model::{CellRange, Doc};

use crate::install;

pub(crate) struct AppCommand;

const USAGE: &str = "Usage: app input [<range> | clear [<range>]] | app install <name> [nosave] [icon=<text>] | app uninstall <name> | app list";

impl Command for AppCommand {
    fn name(&self) -> &str {
        "app"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "App",
            summary: "Mark input cells, and register workbooks as apps",
            usage: "app [input [<range> | clear [<range>]] | install <name> [nosave] [icon=<text>] | uninstall <name> | list]",
            description: "An app is a workbook that opens showing only its grid, where users can change only \
                          the input cells, such as a quote calculator.\n\
                          \n\
                          Making an app:\n\
                          1. Build the form with labels and formulas, on one sheet.\n\
                          2. Mark the cells users fill in: select them and right-click, Mark as input cells, \
                          or type app input B2:B9. They show a light tint, and app input lists them.\n\
                          3. Save as an .xlsx file while the form's sheet is showing. The app opens on that \
                          sheet and cannot switch sheets.\n\
                          4. Type app install and a name, such as app install 견적 계산기. The app appears in \
                          the app list and can be added to the dock. Its icon shows the name's first two \
                          letters, or the text after icon=.\n\
                          \n\
                          Saving: without nosave, the app saves what users type into the same file, so the \
                          next start shows it. With nosave nothing is ever saved, so each start is fresh.\n\
                          \n\
                          Changing an app: open its file as usual, such as kxcel quote.xlsx, change it, and \
                          save. The app shows the change on its next start, without installing again. After \
                          moving or renaming the file, install again.\n\
                          \n\
                          app input clear unmarks every input cell on the sheet, and app input clear <range> \
                          unmarks some. Outside app mode every cell can still be changed. Input cells are \
                          saved as Excel's unlocked cells. app uninstall removes an app and keeps its file, and \
                          app list shows the apps with their files. kxcel --app <file> opens any file as an \
                          app without installing it.",
            args: &[
                ("input <range>", "Marks the range as input cells"),
                ("input clear [<range>]", "Unmarks the range, or every input cell on the sheet"),
                ("install <name>", "Registers the saved workbook as an app with this name"),
                ("nosave", "Optional, after install. The app never saves"),
                ("icon=<text>", "Optional, after install. The icon's letters, such as icon=견"),
                ("uninstall <name>", "Removes a registered app. The file stays"),
                ("list", "Lists registered apps"),
            ],
            examples: &[
                ("app input B2:B10", ""),
                ("app input clear B5", ""),
                ("app install 견적 계산기 nosave", ""),
                ("app install Budget icon=$", ""),
                ("app uninstall 견적 계산기", ""),
                ("app list", ""),
            ],
            ..Doc::BLANK
        }
    }

    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError> {
        match args {
            ["input"] => list(ctx),
            ["input", "clear"] => change(ctx, None, false),
            ["input", "clear", range] => change(ctx, Some(parse(range)?), false),
            ["input", range] => change(ctx, Some(parse(range)?), true),
            ["install", words @ ..] if !words.is_empty() => install(ctx, words),
            ["uninstall", words @ ..] if !words.is_empty() => uninstall(ctx, &words.join(" ")),
            ["list"] => list_apps(ctx),
            _ => Err(CommandError::new(USAGE)),
        }
    }
}

/// Where entries and icons go, or an error saying why there is nowhere.
fn data_dir() -> Result<std::path::PathBuf, CommandError> {
    install::data_dir().ok_or_else(|| CommandError::new("Cannot find the folder for app entries: HOME is not set"))
}

/// Reads `<name> [nosave] [icon=<text>]`, options last, and registers the
/// open workbook, which must be saved, as an app.
fn install(ctx: &mut Context<'_>, words: &[&str]) -> Result<(), CommandError> {
    let mut words = words.to_vec();
    let (mut nosave, mut icon) = (false, None);
    while let Some(last) = words.last() {
        if last.eq_ignore_ascii_case("nosave") {
            nosave = true;
        } else if let Some(text) = last.strip_prefix("icon=") {
            icon = Some(text.to_owned());
        } else {
            break;
        }
        words.pop();
    }
    let name = words.join(" ");
    if name.trim().is_empty() {
        return Err(CommandError::new("Usage: app install <name> [nosave] [icon=<text>]"));
    }
    let Some(path) = ctx.doc.path.clone().filter(|path| path.exists()) else {
        return Err(CommandError::new("Save the workbook first, so the app has a file to open"));
    };
    if ctx.doc.dirty {
        return Err(CommandError::new("Save the workbook first, so the app opens what you see"));
    }
    let file = path.canonicalize().unwrap_or(path);
    let program = install::app_program().map_err(|e| CommandError::new(format!("Cannot find this program: {e}")))?;
    let data = data_dir()?;
    let request = install::Request { name: name.clone(), file: file.clone(), nosave, icon };
    let replaced = install::install(&data, &program, &request)
        .map_err(|e| CommandError::new(format!("Cannot register {name}: {e}")))?;
    install::refresh(&data);
    let verb = if replaced { "Updated" } else { "Registered" };
    let mut message = format!("{verb} app {name}, which opens {} in app mode", file.display());
    let workbook = &ctx.doc.workbook;
    if workbook.sheets().iter().all(|sheet| sheet.meta().input_cells.is_empty()) {
        message.push_str(". It has no input cells yet: app input <range> marks some, then save");
    } else if workbook.active().meta().input_cells.is_empty() {
        message.push_str(&format!(
            ". It opens on sheet {}, which has no input cells: switch to the sheet with them, then save",
            workbook.active().name
        ));
    }
    ctx.say(message);
    Ok(())
}

fn uninstall(ctx: &mut Context<'_>, name: &str) -> Result<(), CommandError> {
    let data = data_dir()?;
    let removed =
        install::uninstall(&data, name).map_err(|e| CommandError::new(format!("Cannot remove {name}: {e}")))?;
    if !removed {
        return Err(CommandError::new(format!("No app named {name}. app list shows them")));
    }
    install::refresh(&data);
    ctx.say(format!("Removed app {name}. Its file stays"));
    Ok(())
}

fn list_apps(ctx: &mut Context<'_>) -> Result<(), CommandError> {
    let apps = install::list(&data_dir()?);
    if apps.is_empty() {
        ctx.say("No apps registered. app install <name> registers the open workbook");
        return Ok(());
    }
    let shown: Vec<String> = apps
        .iter()
        .map(|app| {
            let file =
                app.file.file_name().map_or_else(|| app.file.display().to_string(), |n| n.to_string_lossy().into());
            let nosave = if app.nosave { ", nosave" } else { "" };
            format!("{} ({file}{nosave})", app.name)
        })
        .collect();
    ctx.say(shown.join("  "));
    Ok(())
}

fn parse(text: &str) -> Result<CellRange, CommandError> {
    text.parse().map_err(CommandError::new)
}

fn list(ctx: &mut Context<'_>) -> Result<(), CommandError> {
    let ranges = ctx.doc.workbook.active().meta().input_cells.ranges();
    if ranges.is_empty() {
        ctx.say("No input cells on this sheet. app input <range> marks some");
    } else {
        let names: Vec<String> = ranges.iter().map(ToString::to_string).collect();
        ctx.say(format!("Input cells: {}", names.join(", ")));
    }
    Ok(())
}

/// Marks or unmarks input cells. `None` with `add` false unmarks them all.
fn change(ctx: &mut Context<'_>, range: Option<CellRange>, add: bool) -> Result<(), CommandError> {
    let index = ctx.doc.workbook.active_index();
    let before = ctx.doc.workbook.active().meta().clone();
    let mut after = before.clone();
    let message = match range {
        Some(range) if add => {
            after.input_cells.add(range);
            format!("{range} is now input cells")
        }
        Some(range) => {
            after.input_cells.remove(range);
            format!("{range} is no longer input cells")
        }
        None => {
            after.input_cells.clear();
            "No input cells on this sheet now".to_owned()
        }
    };
    if after != before {
        ctx.apply(Edit::SetMeta { sheet: index, before, after });
    }
    ctx.say(message);
    Ok(())
}

#[cfg(test)]
mod tests {
    use excel_command::testing::{NoCalculator, NoStore};
    use excel_edit::Document;

    use super::*;

    fn run(doc: &mut Document, args: &[&str]) -> Result<String, CommandError> {
        let mut ctx = Context::new(doc, &NoCalculator, &NoStore);
        AppCommand.run(&mut ctx, args)?;
        Ok(ctx.message.unwrap_or_default())
    }

    #[test]
    fn marks_lists_and_clears_input_cells() {
        let mut d = Document::default();
        assert!(run(&mut d, &["input"]).unwrap().starts_with("No input cells"));
        run(&mut d, &["input", "B2:B5"]).unwrap();
        run(&mut d, &["input", "D1"]).unwrap();
        assert_eq!(run(&mut d, &["input"]).unwrap(), "Input cells: D1, B2:B5");
        run(&mut d, &["input", "clear", "B3"]).unwrap();
        assert_eq!(run(&mut d, &["input"]).unwrap(), "Input cells: D1, B2, B4:B5");
        run(&mut d, &["input", "clear"]).unwrap();
        assert!(d.workbook.active().meta().input_cells.is_empty());
    }

    #[test]
    fn each_change_is_one_undo_step() {
        let mut d = Document::default();
        run(&mut d, &["input", "A1:A3"]).unwrap();
        run(&mut d, &["input", "clear", "A2"]).unwrap();
        d.undo();
        assert!(d.workbook.active().is_input("A2".parse().unwrap()));
        d.undo();
        assert!(!d.workbook.active().is_input("A1".parse().unwrap()));
    }

    #[test]
    fn install_needs_a_saved_workbook() {
        let mut d = Document::default();
        assert!(run(&mut d, &["install", "Budget"]).unwrap_err().message.contains("Save the workbook first"));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("b.xlsx");
        std::fs::write(&path, b"x").unwrap();
        d.path = Some(path);
        d.dirty = true;
        assert!(run(&mut d, &["install", "Budget"]).unwrap_err().message.contains("what you see"));
    }

    #[test]
    fn rejects_bad_arguments() {
        let mut d = Document::default();
        for args in
            [&[][..], &["input", "nope"], &["output"], &["input", "A1", "B2"], &["install"], &["install", "nosave"]]
        {
            assert!(run(&mut d, args).is_err(), "{args:?}");
        }
    }
}
