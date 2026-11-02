use calamine::Range;
use excel_model::{Cell, CellAddr, CellValue, Sheet};

/// Turns cells with formulas into formula cells, keeping their stored results.
pub(super) fn add_formulas(sheet: &mut Sheet, range: &Range<String>) {
    let Some((first_row, first_col)) = range.start() else {
        return;
    };
    for (row, col, formula) in range.used_cells() {
        if formula.is_empty() {
            continue;
        }
        let addr = CellAddr::new(first_row + row as u32, first_col + col as u32);
        let stored = sheet.get(addr).map_or(CellValue::Empty, |cell| cell.value.clone());
        sheet.set(addr, Some(Cell::formula(formula.clone(), stored)));
    }
}
