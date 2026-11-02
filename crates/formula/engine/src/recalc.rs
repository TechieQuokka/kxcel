use std::collections::HashMap;

use excel_formula_functions::FunctionRegistry;
use excel_formula_syntax::{Expr, parse};
use excel_model::{CellError, CellValue, Workbook};

use crate::deps::{Node, plan};
use crate::eval::Evaluator;

/// Recomputes every formula in the workbook.
///
/// A formula the engine cannot run, because it has invalid syntax or an unknown
/// function, keeps its stored value and is marked `uncalculated`. Formulas that
/// read it use that stored value.
pub(crate) fn recalculate(workbook: &mut Workbook, functions: &FunctionRegistry) {
    let formulas = parse_formulas(workbook, functions);
    let plan = plan(workbook, &formulas);

    for (sheet, addr) in &plan.cycles {
        set_value(workbook, (*sheet, *addr), CellValue::Error(CellError::Cycle));
    }
    for node in plan.order {
        let value = Evaluator { workbook, functions, sheet: node.0 }.eval(&formulas[&node]);
        set_value(workbook, node, value);
    }
}

fn set_value(workbook: &mut Workbook, (sheet, addr): Node, value: CellValue) {
    if let Some(cell) = workbook.sheets_mut()[sheet].get_mut(addr) {
        cell.value = value;
    }
}

/// Parses every formula the engine can run, and marks the rest `uncalculated`.
fn parse_formulas(workbook: &mut Workbook, functions: &FunctionRegistry) -> HashMap<Node, Expr> {
    let mut formulas = HashMap::new();
    for (index, sheet) in workbook.sheets_mut().iter_mut().enumerate() {
        for (addr, cell) in sheet.iter_mut() {
            let Some(text) = cell.formula_text() else {
                continue;
            };
            match parse(text).ok().filter(|expr| only_known_functions(expr, functions)) {
                Some(expr) => {
                    cell.uncalculated = false;
                    formulas.insert((index, addr), expr);
                }
                None => cell.uncalculated = true,
            }
        }
    }
    formulas
}

fn only_known_functions(expr: &Expr, functions: &FunctionRegistry) -> bool {
    let mut known = true;
    expr.walk(&mut |part| {
        if let Expr::Call { name, .. } = part {
            known &= functions.get(name).is_some();
        }
    });
    known
}
