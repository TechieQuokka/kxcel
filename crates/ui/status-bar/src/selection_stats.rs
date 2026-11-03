use excel_model::{CellAddr, CellRange, CellValue, Sheet};

/// Calculations shown for a selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionStats {
    pub sum: f64,
    pub average: f64,
    pub count: usize,
}

/// Summarizes the numbers in a range, leaving out hidden rows. Returns `None`
/// for a single cell or a range without numbers, where there is nothing useful
/// to show.
pub fn selection_stats(sheet: &Sheet, range: CellRange) -> Option<SelectionStats> {
    if range.is_single() {
        return None;
    }
    let (sum, count) = sheet
        .iter_range(range)
        .filter(|(addr, _)| !sheet.is_row_hidden(addr.row))
        .filter_map(|(_, cell)| match cell.value {
            CellValue::Number(n) => Some(n),
            _ => None,
        })
        .fold((0.0, 0), |(sum, count), n| (sum + n, count + 1));
    (count > 0).then(|| SelectionStats { sum, average: sum / count as f64, count })
}

/// How many of a filtered table's data rows show, such as `12 of 340 rows`,
/// for the table at `cursor`. `None` outside a table or for one without filters.
pub fn table_rows(sheet: &Sheet, cursor: CellAddr) -> Option<String> {
    let table = sheet.table_at(cursor).filter(|table| !table.filters.is_empty())?;
    let total = u64::from(table.range.rows() - 1);
    let hidden =
        if total == 0 { 0 } else { sheet.hidden_rows().count_within(table.range.start.row + 1, table.range.end.row) };
    Some(format!("{} of {total} rows", total - hidden))
}

#[cfg(test)]
mod tests {
    use excel_model::Cell;

    use super::*;

    fn sheet() -> Sheet {
        let mut sheet = Sheet::new("S");
        for (addr, value) in [
            ("A1", CellValue::Number(1.0)),
            ("A2", CellValue::Number(3.0)),
            ("A3", CellValue::Text("x".into())),
            ("B1", CellValue::Number(100.0)),
        ] {
            sheet.set(addr.parse().unwrap(), Some(Cell::literal(value)));
        }
        sheet
    }

    #[test]
    fn counts_numbers_only() {
        let stats = selection_stats(&sheet(), "A1:A4".parse().unwrap()).unwrap();
        assert_eq!(stats, SelectionStats { sum: 4.0, average: 2.0, count: 2 });
    }

    #[test]
    fn hidden_rows_are_left_out() {
        let mut sheet = sheet();
        let mut meta = sheet.meta().clone();
        meta.hidden_rows.insert(1);
        sheet.set_meta(meta);
        let stats = selection_stats(&sheet, "A1:A4".parse().unwrap()).unwrap();
        assert_eq!(stats, SelectionStats { sum: 1.0, average: 1.0, count: 1 });
    }

    #[test]
    fn filtered_tables_count_their_shown_rows() {
        let mut sheet = sheet();
        let mut meta = sheet.meta().clone();
        let mut table = excel_model::Table::new("T", "A1:B5".parse().unwrap());
        assert_eq!(table_rows(&Sheet::new("S"), "A2".parse().unwrap()), None);
        table
            .filters
            .push(excel_model::ColumnFilter { column: 0, rule: excel_model::FilterRule::Condition(">0".into()) });
        meta.tables.push(table);
        meta.hidden_rows.insert_run(2, 3);
        sheet.set_meta(meta);
        assert_eq!(table_rows(&sheet, "B2".parse().unwrap()).as_deref(), Some("2 of 4 rows"));
        assert_eq!(table_rows(&sheet, "C2".parse().unwrap()), None);
    }

    #[test]
    fn single_cells_have_no_stats() {
        assert_eq!(selection_stats(&sheet(), "A1".parse().unwrap()), None);
    }

    #[test]
    fn ranges_without_numbers_have_no_stats() {
        assert_eq!(selection_stats(&sheet(), "C1:C9".parse().unwrap()), None);
    }
}
