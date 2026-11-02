use excel_command::{Context, Doc, Effect};
use excel_features::{COMMAND_CATEGORIES, FUNCTION_CATEGORIES, Services, registry};
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
                Effect::Fullscreen(_) => eprintln!("{line}: full screen only applies in the app window"),
                Effect::Theme(_) => eprintln!("{line}: themes only apply in the app window"),
                _ => {}
            }
        }
    }

    if args.print {
        print!("{}", to_tsv(doc.workbook.active()));
    }
    if doc.dirty {
        eprintln!("Warning: changes were not saved. Add -c save to save them");
    }
    Ok(())
}

/// Help for `topic`: a command's or function's whole entry when the topic names
/// one exactly, otherwise the commands and functions containing it, by category.
pub fn help_text(topic: Option<&str>) -> String {
    let topic = topic.unwrap_or_default().trim();
    let commands = registry().docs();
    let functions = Services::default().calculator.functions();

    let exact_command = commands.iter().find(|(name, _)| name.eq_ignore_ascii_case(topic));
    let bare = topic.strip_prefix('=').unwrap_or(topic);
    let exact_function = functions.iter().find(|(name, _)| name.eq_ignore_ascii_case(bare));
    match (exact_command, exact_function) {
        (Some((_, doc)), None) => return doc.to_text(),
        (None, Some((_, doc))) => return format!("={}", doc.to_text()),
        (Some((_, command)), Some((_, function))) => {
            return format!("{}\n={}", command.to_text(), function.to_text());
        }
        (None, None) => {}
    }

    let needle = topic.to_lowercase();
    let matches = |name: &str, doc: &Doc| {
        needle.is_empty() || name.to_lowercase().contains(&needle) || doc.summary.to_lowercase().contains(&needle)
    };
    let mut text = String::new();
    let commands: Vec<_> =
        commands.iter().filter(|(name, doc)| matches(name, doc)).map(|(n, d)| (n.as_str(), *d)).collect();
    if !commands.is_empty() {
        text.push_str("Commands (run with -c, or Ctrl+L in the app):\n");
        push_grouped(&mut text, &commands, COMMAND_CATEGORIES, "");
    }
    let functions: Vec<_> = functions.into_iter().filter(|(name, doc)| matches(name, doc)).collect();
    if !functions.is_empty() {
        text.push_str("Functions (type = in a cell, or set A1 =SUM(...)):\n");
        push_grouped(&mut text, &functions, FUNCTION_CATEGORIES, "=");
    }
    if text.is_empty() {
        text.push_str("No matches\n");
    } else {
        text.push_str("Run help <name> for details.\n");
    }
    text
}

/// Lists entries under their categories, in the given order, then any others.
fn push_grouped(text: &mut String, entries: &[(&str, Doc)], order: &[&str], prefix: &str) {
    let mut categories: Vec<&str> = order.to_vec();
    for (_, doc) in entries {
        if !categories.contains(&doc.category) {
            categories.push(doc.category);
        }
    }
    for category in categories {
        let group: Vec<_> = entries.iter().filter(|(_, doc)| doc.category == category).collect();
        if group.is_empty() {
            continue;
        }
        text.push_str(&format!("  {category}\n"));
        for (_, doc) in group {
            let usage = format!("{prefix}{}", doc.usage);
            text.push_str(&format!("    {usage:<36}{}\n", doc.summary));
        }
    }
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
            (0..=used.end.col).map(|col| quote_field(&sheet.display(CellAddr::new(row, col)))).collect();
        text.push_str(&cells.join("\t"));
        text.push('\n');
    }
    text
}
