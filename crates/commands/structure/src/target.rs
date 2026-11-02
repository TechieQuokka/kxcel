use excel_command::{CommandError, Context, Structural};
use excel_edit::Edit;
use excel_model::{CellInput, MAX_COLS, MAX_ROWS, Sheet, col_from_name, col_name};

/// Whether a command acts on rows or columns, at which position, and how many.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Target {
    pub rows: bool,
    pub at: u32,
    pub count: u32,
}

impl Target {
    /// Parses `row|col [position] [count]`. The position defaults to the cursor.
    pub fn parse(ctx: &Context<'_>, args: &[&str], usage: &str) -> Result<Self, CommandError> {
        let usage_error = || CommandError::new(usage);
        let (kind, rest) = args.split_first().ok_or_else(usage_error)?;
        let rows = match kind.to_ascii_lowercase().as_str() {
            "row" | "rows" => true,
            "col" | "cols" | "column" | "columns" => false,
            _ => return Err(usage_error()),
        };

        let at = match rest.first() {
            None => {
                if rows {
                    ctx.cursor.row
                } else {
                    ctx.cursor.col
                }
            }
            Some(text) if rows => text
                .parse::<u32>()
                .ok()
                .filter(|row| (1..=MAX_ROWS).contains(row))
                .map(|row| row - 1)
                .ok_or_else(|| CommandError::new(format!("Invalid row: {text}")))?,
            Some(text) => col_from_name(text).ok_or_else(|| CommandError::new(format!("Invalid column: {text}")))?,
        };

        let limit = if rows { MAX_ROWS } else { MAX_COLS };
        let count = match rest.get(1) {
            None => 1,
            Some(text) => text
                .parse::<u32>()
                .ok()
                .filter(|count| (1..=limit).contains(count))
                .ok_or_else(|| CommandError::new(format!("Invalid count: {text}")))?,
        };
        if rest.len() > 2 {
            return Err(usage_error());
        }
        Ok(Self { rows, at, count: count.min(limit - at) })
    }

    /// Describes the target, such as `2 rows at 3` or `1 column at C`.
    pub fn describe(&self) -> String {
        let plural = if self.count == 1 { "" } else { "s" };
        if self.rows {
            format!("{} row{plural} at {}", self.count, self.at + 1)
        } else {
            format!("{} column{plural} at {}", self.count, col_name(self.at))
        }
    }
}

/// Applies a structural change to the active sheet and rewrites every formula
/// in the workbook that refers to it.
pub(crate) fn apply(ctx: &mut Context<'_>, change: Structural) {
    let active = ctx.doc.workbook.active_index();
    let calculator = ctx.calculator;
    let edit = Edit::replacing_sheets(&ctx.doc.workbook, |sheets| {
        let changed = sheets[active].name.clone();
        restructure(&mut sheets[active], change);
        for sheet in sheets.iter_mut() {
            let own = sheet.name.clone();
            for (_, cell) in sheet.iter_mut() {
                if let CellInput::Formula(text) = &cell.input {
                    cell.input = CellInput::Formula(calculator.adjust(text, &own, &changed, change));
                }
            }
        }
    });
    ctx.apply(edit);
}

fn restructure(sheet: &mut Sheet, change: Structural) {
    match change {
        Structural::InsertRows { at, count } => sheet.insert_rows(at, count),
        Structural::DeleteRows { at, count } => sheet.delete_rows(at, count),
        Structural::InsertCols { at, count } => sheet.insert_cols(at, count),
        Structural::DeleteCols { at, count } => sheet.delete_cols(at, count),
    }
}
