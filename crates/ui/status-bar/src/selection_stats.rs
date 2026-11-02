use excel_model::{CellRange, CellValue, Sheet};

/// Calculations shown for a selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionStats {
    pub sum: f64,
    pub average: f64,
    pub count: usize,
}

/// Summarizes the numbers in a range. Returns `None` for a single cell or a
/// range without numbers, where there is nothing useful to show.
pub fn selection_stats(sheet: &Sheet, range: CellRange) -> Option<SelectionStats> {
    if range.is_single() {
        return None;
    }
    let (sum, count) = sheet
        .iter_range(range)
        .filter_map(|(_, cell)| match cell.value {
            CellValue::Number(n) => Some(n),
            _ => None,
        })
        .fold((0.0, 0), |(sum, count), n| (sum + n, count + 1));
    (count > 0).then(|| SelectionStats { sum, average: sum / count as f64, count })
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
    fn single_cells_have_no_stats() {
        assert_eq!(selection_stats(&sheet(), "A1".parse().unwrap()), None);
    }

    #[test]
    fn ranges_without_numbers_have_no_stats() {
        assert_eq!(selection_stats(&sheet(), "C1:C9".parse().unwrap()), None);
    }
}
