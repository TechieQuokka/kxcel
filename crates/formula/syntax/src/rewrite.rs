use crate::{CellRef, Expr, parse};

const REF_ERROR: &str = "#REF!";

/// Inserted or deleted rows or columns on one sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    InsertRows { at: u32, count: u32 },
    DeleteRows { at: u32, count: u32 },
    InsertCols { at: u32, count: u32 },
    DeleteCols { at: u32, count: u32 },
}

/// Rewrites a formula for a copy moved by an offset. Relative references move
/// with it. Text that does not parse is returned unchanged.
pub fn translate(formula: &str, rows: i64, cols: i64) -> String {
    let Ok(mut expr) = parse(formula) else {
        return formula.to_owned();
    };
    map_references(&mut expr, &mut |reference| shift_reference(reference, rows, cols));
    expr.to_string()
}

/// Rewrites a formula living on `own_sheet` after `change` was made to
/// `changed_sheet`. Only references to that sheet change. Text that does not
/// parse is returned unchanged.
pub fn adjust(formula: &str, own_sheet: &str, changed_sheet: &str, change: Change) -> String {
    let Ok(mut expr) = parse(formula) else {
        return formula.to_owned();
    };
    map_references(&mut expr, &mut |reference| {
        let target = match reference {
            Expr::Cell { sheet, .. } | Expr::Range { sheet, .. } => sheet.as_deref().unwrap_or(own_sheet),
            _ => return reference.clone(),
        };
        if target.eq_ignore_ascii_case(changed_sheet) { adjust_reference(reference, change) } else { reference.clone() }
    });
    expr.to_string()
}

fn map_references(expr: &mut Expr, map: &mut impl FnMut(&Expr) -> Expr) {
    match expr {
        Expr::Cell { .. } | Expr::Range { .. } => *expr = map(expr),
        Expr::Unary(_, inner) => map_references(inner, map),
        Expr::Binary(_, left, right) => {
            map_references(left, map);
            map_references(right, map);
        }
        Expr::Call { args, .. } => args.iter_mut().for_each(|arg| map_references(arg, map)),
        _ => {}
    }
}

fn ref_error() -> Expr {
    Expr::Error(REF_ERROR.to_owned())
}

fn shift_cell(cell: CellRef, rows: i64, cols: i64) -> Option<CellRef> {
    let moved = cell.addr.offset(if cell.abs_row { 0 } else { rows }, if cell.abs_col { 0 } else { cols })?;
    Some(CellRef { addr: moved, ..cell })
}

fn shift_reference(reference: &Expr, rows: i64, cols: i64) -> Expr {
    match reference {
        Expr::Cell { sheet, cell } => match shift_cell(*cell, rows, cols) {
            Some(cell) => Expr::Cell { sheet: sheet.clone(), cell },
            None => ref_error(),
        },
        Expr::Range { sheet, start, end } => match (shift_cell(*start, rows, cols), shift_cell(*end, rows, cols)) {
            (Some(start), Some(end)) => Expr::Range { sheet: sheet.clone(), start, end },
            _ => ref_error(),
        },
        other => other.clone(),
    }
}

/// Maps a position along the changed axis. Returns `None` if it was deleted.
fn map_position(position: u32, at: u32, count: u32, insert: bool) -> Option<u32> {
    if position < at {
        Some(position)
    } else if insert {
        Some(position.saturating_add(count))
    } else if position < at.saturating_add(count) {
        None
    } else {
        Some(position - count)
    }
}

/// Maps a range's span. A range keeps whatever survives a deletion.
fn map_span(first: u32, last: u32, at: u32, count: u32, insert: bool) -> Option<(u32, u32)> {
    if insert {
        return Some((map_position(first, at, count, true)?, map_position(last, at, count, true)?));
    }
    let new_first = map_position(first, at, count, false).unwrap_or(at);
    let new_last = map_position(last, at, count, false).unwrap_or_else(|| at.wrapping_sub(1));
    let survives = last >= first && (first < at || last >= at.saturating_add(count));
    survives.then_some((new_first, new_last))
}

fn adjust_reference(reference: &Expr, change: Change) -> Expr {
    let (at, count, insert, along_rows) = match change {
        Change::InsertRows { at, count } => (at, count, true, true),
        Change::DeleteRows { at, count } => (at, count, false, true),
        Change::InsertCols { at, count } => (at, count, true, false),
        Change::DeleteCols { at, count } => (at, count, false, false),
    };
    let set = |cell: CellRef, position: u32| -> Option<CellRef> {
        let mut moved = cell;
        if along_rows {
            moved.addr.row = position;
        } else {
            moved.addr.col = position;
        }
        // A reference pushed past the sheet's last row or column is gone.
        let limit = if along_rows { excel_model::MAX_ROWS } else { excel_model::MAX_COLS };
        (position < limit).then_some(moved)
    };
    let along = |cell: CellRef| if along_rows { cell.addr.row } else { cell.addr.col };

    match reference {
        Expr::Cell { sheet, cell } => map_position(along(*cell), at, count, insert)
            .and_then(|position| set(*cell, position))
            .map_or_else(ref_error, |cell| Expr::Cell { sheet: sheet.clone(), cell }),
        Expr::Range { sheet, start, end } => {
            let (first, last) = (along(*start).min(along(*end)), along(*start).max(along(*end)));
            let Some((new_first, new_last)) = map_span(first, last, at, count, insert) else {
                return ref_error();
            };
            let (new_start, new_end) =
                if along(*start) <= along(*end) { (new_first, new_last) } else { (new_last, new_first) };
            match (set(*start, new_start), set(*end, new_end)) {
                (Some(start), Some(end)) => Expr::Range { sheet: sheet.clone(), start, end },
                _ => ref_error(),
            }
        }
        other => other.clone(),
    }
}
