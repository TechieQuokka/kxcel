//! What the status bar works out from the sheet, kept until it may change.

use excel_model::{CellAddr, CellRange, Sheet};

use crate::{SelectionStats, selection_stats, table_rows};

/// The sum, average, and count of the selection, and the shown rows of a
/// filtered table at the cursor.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Summary {
    pub stats: Option<SelectionStats>,
    pub table_rows: Option<String>,
}

/// What a summary was worked out for: the workbook and its revision, the
/// sheet, the selection, and the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryKey {
    pub workbook: u64,
    pub revision: u64,
    pub sheet: usize,
    pub selection: CellRange,
    pub cursor: CellAddr,
}

/// Keeps the latest summary, so a large selection is added up once rather
/// than on every frame.
#[derive(Debug, Default)]
pub struct SummaryCache {
    key: Option<SummaryKey>,
    summary: Summary,
    /// How many times a summary was worked out, for tests.
    computed: usize,
}

impl SummaryCache {
    /// The summary for `key`, working it out only when the key changed.
    pub fn get(&mut self, key: SummaryKey, sheet: &Sheet) -> &Summary {
        if self.key != Some(key) {
            self.summary =
                Summary { stats: selection_stats(sheet, key.selection), table_rows: table_rows(sheet, key.cursor) };
            self.key = Some(key);
            self.computed += 1;
        }
        &self.summary
    }

    /// How many times a summary was worked out.
    pub fn computed(&self) -> usize {
        self.computed
    }
}

#[cfg(test)]
mod tests {
    use excel_model::Cell;

    use super::*;

    #[test]
    fn works_out_again_only_when_something_changed() {
        let mut sheet = Sheet::new("S");
        sheet.set("A1".parse().unwrap(), Cell::parse_input("2"));
        sheet.set("A2".parse().unwrap(), Cell::parse_input("3"));
        let key = SummaryKey {
            workbook: 1,
            revision: 0,
            sheet: 0,
            selection: "A1:A9".parse().unwrap(),
            cursor: CellAddr::default(),
        };
        let mut cache = SummaryCache::default();
        assert_eq!(cache.get(key, &sheet).stats.unwrap().sum, 5.0);
        cache.get(key, &sheet);
        assert_eq!(cache.computed(), 1);
        sheet.set("A3".parse().unwrap(), Cell::parse_input("5"));
        assert_eq!(cache.get(SummaryKey { revision: 1, ..key }, &sheet).stats.unwrap().sum, 10.0);
        cache.get(SummaryKey { selection: "A1:A2".parse().unwrap(), revision: 1, ..key }, &sheet);
        cache.get(SummaryKey { workbook: 2, revision: 1, ..key }, &sheet);
        assert_eq!(cache.computed(), 4, "a new revision, selection, or workbook each count");
    }
}
