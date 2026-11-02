use excel_command::{Calculator, Structural};
use excel_formula_functions::FunctionRegistry;
use excel_formula_syntax::{Change, adjust, translate};
use excel_model::Workbook;

use crate::recalc::recalculate;

/// The formula engine, supplied to commands as their [`Calculator`].
pub struct FormulaEngine {
    functions: FunctionRegistry,
}

impl Default for FormulaEngine {
    fn default() -> Self {
        Self { functions: FunctionRegistry::with_builtins() }
    }
}

impl FormulaEngine {
    /// Every function's name and help, sorted by name.
    pub fn functions(&self) -> Vec<(&'static str, &'static str)> {
        self.functions.list()
    }
}

impl Calculator for FormulaEngine {
    fn recalculate(&self, workbook: &mut Workbook) {
        recalculate(workbook, &self.functions);
    }

    fn translate(&self, formula: &str, rows: i64, cols: i64) -> String {
        translate(formula, rows, cols)
    }

    fn adjust(&self, formula: &str, own_sheet: &str, changed_sheet: &str, change: Structural) -> String {
        let change = match change {
            Structural::InsertRows { at, count } => Change::InsertRows { at, count },
            Structural::DeleteRows { at, count } => Change::DeleteRows { at, count },
            Structural::InsertCols { at, count } => Change::InsertCols { at, count },
            Structural::DeleteCols { at, count } => Change::DeleteCols { at, count },
        };
        adjust(formula, own_sheet, changed_sheet, change)
    }
}
