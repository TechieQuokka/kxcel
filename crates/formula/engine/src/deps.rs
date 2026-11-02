use std::collections::{BTreeSet, HashMap, VecDeque};

use excel_formula_syntax::Expr;
use excel_model::{CellAddr, CellRange, Workbook};

/// A formula cell: its sheet index and address.
pub(crate) type Node = (usize, CellAddr);

/// The order to compute formulas in, and the formulas stuck in cycles.
#[derive(Debug, Default)]
pub(crate) struct Plan {
    pub order: Vec<Node>,
    pub cycles: Vec<Node>,
}

/// Sorts formulas so that each is computed after the formulas it reads.
///
/// This is iterative, so long dependency chains cannot overflow the stack.
pub(crate) fn plan(workbook: &Workbook, formulas: &HashMap<Node, Expr>) -> Plan {
    // Formula addresses per sheet, for finding the formulas inside a range.
    let mut per_sheet: Vec<BTreeSet<CellAddr>> = vec![BTreeSet::new(); workbook.sheets().len()];
    for (sheet, addr) in formulas.keys() {
        per_sheet[*sheet].insert(*addr);
    }

    let mut dependents: HashMap<Node, Vec<Node>> = HashMap::new();
    let mut waiting_on: HashMap<Node, usize> = formulas.keys().map(|node| (*node, 0)).collect();

    for (&node, expr) in formulas {
        let mut sources = Vec::new();
        expr.walk(&mut |part| collect_sources(part, node.0, workbook, &per_sheet, &mut sources));
        sources.sort_unstable();
        sources.dedup();
        for source in sources {
            dependents.entry(source).or_default().push(node);
            *waiting_on.entry(node).or_default() += 1;
        }
    }

    let mut ready: VecDeque<Node> =
        waiting_on.iter().filter(|(_, count)| **count == 0).map(|(node, _)| *node).collect();
    let mut order = Vec::with_capacity(formulas.len());
    while let Some(node) = ready.pop_front() {
        order.push(node);
        for dependent in dependents.get(&node).into_iter().flatten() {
            let count = waiting_on.get_mut(dependent).expect("every dependent is a formula");
            *count -= 1;
            if *count == 0 {
                ready.push_back(*dependent);
            }
        }
    }

    // Whatever never became ready is in a cycle or depends on one.
    let cycles = waiting_on.into_iter().filter(|(_, count)| *count > 0).map(|(node, _)| node).collect();
    Plan { order, cycles }
}

fn collect_sources(
    part: &Expr,
    own_sheet: usize,
    workbook: &Workbook,
    per_sheet: &[BTreeSet<CellAddr>],
    sources: &mut Vec<Node>,
) {
    let resolve = |sheet: &Option<String>| match sheet {
        Some(name) => workbook.sheet_index(name),
        None => Some(own_sheet),
    };
    match part {
        Expr::Cell { sheet, cell } => {
            if let Some(index) = resolve(sheet)
                && per_sheet[index].contains(&cell.addr)
            {
                sources.push((index, cell.addr));
            }
        }
        Expr::Range { sheet, start, end } => {
            if let Some(index) = resolve(sheet) {
                let range = CellRange::new(start.addr, end.addr);
                let inside = per_sheet[index].range(range.start..=range.end).filter(|addr| range.contains(**addr));
                sources.extend(inside.map(|addr| (index, *addr)));
            }
        }
        _ => {}
    }
}
