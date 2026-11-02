use std::cell::RefCell;

use excel_command::{Calculator, Structural};
use excel_edit::Edit;
use excel_formula_functions::FunctionRegistry;
use excel_formula_syntax::{Change, adjust, translate};
use excel_model::{Doc, Workbook};

use crate::graph::Graph;

/// The formula engine, supplied to commands as their [`Calculator`].
///
/// It keeps the dependency graph of the workbook it last calculated, so an
/// edit to some cells recomputes only the formulas that depend on them. Any
/// other workbook, or a change to whole sheets, is calculated from scratch.
pub struct FormulaEngine {
    functions: FunctionRegistry,
    graph: RefCell<Option<Graph>>,
}

impl Default for FormulaEngine {
    fn default() -> Self {
        Self { functions: FunctionRegistry::with_builtins(), graph: RefCell::new(None) }
    }
}

impl FormulaEngine {
    /// Every function's name and help, sorted by name.
    pub fn functions(&self) -> Vec<(&'static str, Doc)> {
        self.functions.list()
    }
}

impl Calculator for FormulaEngine {
    fn recalculate(&self, workbook: &mut Workbook) {
        *self.graph.borrow_mut() = Some(Graph::calculate_all(workbook, &self.functions));
    }

    fn update(&self, workbook: &mut Workbook, edit: &Edit) {
        let Edit::SetCells(changes) = edit else {
            return self.recalculate(workbook);
        };
        let mut slot = self.graph.borrow_mut();
        let graph = match slot.take() {
            Some(graph) if graph.fits(workbook) => graph,
            _ => {
                drop(slot);
                return self.recalculate(workbook);
            }
        };
        let formula =
            |cell: &Option<excel_model::Cell>| cell.as_ref().and_then(|c| c.formula_text()).map(str::to_owned);
        let changed: Vec<_> = changes
            .iter()
            .map(|change| ((change.sheet, change.addr), formula(&change.before), formula(&change.after)))
            .collect();
        *slot = Some(graph.update(workbook, &self.functions, &changed));
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
