use excel_command::{Context, Effect};
use excel_features::{Services, registry};
use excel_model::{CellAddr, CellRange, Sheet};

use crate::args::Args;

/// Runs the requested commands. The cursor persists between them, so
/// `goto B5` followed by `insert row` acts at row 5.
pub fn run(args: &Args) -> Result<(), String> {
    let services = Services::default();
    let registry = registry();
    let mut doc = services.open(&args.file).map_err(|error| error.to_string())?;
    if !doc.loss_warnings.is_empty() {
        eprintln!("Warning: saving over this file would lose: {}", doc.loss_warnings.join(", "));
    }

    let mut cursor = CellAddr::default();
    'commands: for line in &args.commands {
        let mut ctx = Context::new(&mut doc, &services.calculator, &services.store);
        ctx.cursor = cursor;
        ctx.selection = CellRange::single(cursor);
        registry.execute(line, &mut ctx).map_err(|error| format!("{line}: {error}"))?;
        if let Some(message) = ctx.message.take() {
            println!("{message}");
        }
        let effects = std::mem::take(&mut ctx.effects);
        for effect in effects {
            match effect {
                Effect::MoveCursor(addr) => cursor = addr,
                Effect::DocumentReplaced => cursor = CellAddr::default(),
                Effect::Quit => break 'commands,
                Effect::ShowHelp(topic) => print!("{}", help_text(topic.as_deref())),
                _ => {}
            }
        }
    }

    if args.print {
        print!("{}", to_tsv(doc.workbook.active()));
    }
    if doc.dirty {
        eprintln!("Warning: changes were not saved. Add -c w to save them");
    }
    Ok(())
}

/// Commands and functions, filtered to those containing `topic` if given.
pub fn help_text(topic: Option<&str>) -> String {
    let topic = topic.unwrap_or_default().to_lowercase();
    let matches = |name: &str, usage: &str| {
        topic.is_empty() || name.to_lowercase().contains(&topic) || usage.to_lowercase().contains(&topic)
    };
    let mut text = String::new();
    let commands: Vec<_> = registry().complete("").into_iter().filter(|c| matches(&c.name, &c.help)).collect();
    if !commands.is_empty() {
        text.push_str("Commands (run with -c, or Ctrl+L in the app):\n");
        for command in commands {
            text.push_str(&format!("  {:<8}{}\n", command.name, command.help));
        }
    }
    let functions: Vec<_> =
        Services::default().calculator.functions().into_iter().filter(|(name, help)| matches(name, help)).collect();
    if !functions.is_empty() {
        text.push_str("Functions (type = in a cell, or set A1 =SUM(...)):\n");
        for (_, help) in functions {
            text.push_str(&format!("  ={help}\n"));
        }
    }
    if text.is_empty() {
        text.push_str("No matches\n");
    }
    text
}

/// Quotes a field holding a tab, a line break, or a quote, as spreadsheets do.
fn quote_field(value: &str) -> String {
    if value.contains(['\t', '\n', '\r', '"']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

/// The sheet's used cells as tab-separated values, starting from A1.
fn to_tsv(sheet: &Sheet) -> String {
    let Some(used) = sheet.used_range() else {
        return String::new();
    };
    let mut text = String::new();
    for row in 0..=used.end.row {
        let cells: Vec<String> =
            (0..=used.end.col).map(|col| quote_field(&sheet.value(CellAddr::new(row, col)).to_string())).collect();
        text.push_str(&cells.join("\t"));
        text.push('\n');
    }
    text
}
