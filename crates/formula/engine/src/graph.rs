//! The dependency graph between formulas, kept between edits so that an edit
//! recomputes only the formulas it affects.
//!
//! The graph records what each formula reads, by single references and by
//! ranges, and the edges between formulas. An edit to values finds the formulas
//! reading the changed cells and follows the edges from there. An edit to
//! formulas patches just their edges first. The affected formulas are then
//! sorted among themselves and computed, and those left over form cycles.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use excel_formula_functions::FunctionRegistry;
use excel_formula_syntax::{Expr, parse};
use excel_model::{CellAddr, CellError, CellRange, CellValue, NumberFormat, Workbook};

use crate::eval::Evaluator;

/// A cell: its sheet index and address.
pub(crate) type Node = (usize, CellAddr);

/// Changing more formulas than this at once rebuilds the graph, which is then
/// quicker than patching it one formula at a time.
const MAX_PATCHED: usize = 256;

/// Something a formula reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Read {
    Cell(Node),
    Range(usize, CellRange),
}

/// Every runnable formula in a workbook, and what each reads.
pub(crate) struct Graph {
    /// The workbook this was built for, by [`Workbook::id`].
    workbook: u64,
    /// Sheet names, since references find sheets by name.
    sheets: Vec<String>,
    formulas: HashMap<Node, Expr>,
    /// Formula addresses per sheet, for finding the formulas inside a range.
    per_sheet: Vec<BTreeSet<CellAddr>>,
    /// Formulas that call a function whose result changes on its own, such as `TODAY`.
    volatile: HashSet<Node>,
    reads: HashMap<Node, Vec<Read>>,
    /// For each formula, the formulas it reads.
    sources: HashMap<Node, Vec<Node>>,
    /// For each formula, the formulas that read it.
    dependents: HashMap<Node, Vec<Node>>,
    /// For each cell, the formulas that read it by a single reference.
    cell_readers: HashMap<Node, Vec<Node>>,
    /// Ranges formulas read, with the formula reading each.
    range_readers: Vec<(usize, CellRange, Node)>,
    /// Formulas in a cycle, or depending on one.
    cycles: HashSet<Node>,
}

impl Graph {
    /// Builds the graph and computes every formula.
    pub fn calculate_all(workbook: &mut Workbook, functions: &FunctionRegistry) -> Self {
        let mut graph = Self {
            workbook: workbook.id(),
            sheets: workbook.sheets().iter().map(|sheet| sheet.name.clone()).collect(),
            formulas: HashMap::new(),
            per_sheet: vec![BTreeSet::new(); workbook.sheets().len()],
            volatile: HashSet::new(),
            reads: HashMap::new(),
            sources: HashMap::new(),
            dependents: HashMap::new(),
            cell_readers: HashMap::new(),
            range_readers: Vec::new(),
            cycles: HashSet::new(),
        };
        let mut nodes = Vec::new();
        for (index, sheet) in workbook.sheets().iter().enumerate() {
            nodes.extend(sheet.iter().filter(|(_, cell)| cell.formula_text().is_some()).map(|(addr, _)| (index, addr)));
        }
        let added = graph.insert_formulas(workbook, functions, &nodes);
        // Every formula is new, so their sources give every edge.
        for node in &added {
            for source in graph.sources[node].clone() {
                graph.dependents.entry(source).or_default().push(*node);
            }
        }
        graph.compute(workbook, functions, added);
        graph
    }

    /// Whether this graph still describes `workbook`, so it can be updated.
    pub fn fits(&self, workbook: &Workbook) -> bool {
        self.workbook == workbook.id()
            && self.sheets.len() == workbook.sheets().len()
            && self.sheets.iter().zip(workbook.sheets()).all(|(name, sheet)| *name == sheet.name)
    }

    /// Brings formulas up to date after `changed` cells were set. Each change
    /// gives the formula text before and after, which tells whether the
    /// graph's shape changed.
    pub fn update(
        mut self,
        workbook: &mut Workbook,
        functions: &FunctionRegistry,
        changed: &[(Node, Option<String>, Option<String>)],
    ) -> Self {
        // A cell may change more than once in one edit, so each is handled once.
        let mut reshaped: Vec<Node> =
            changed.iter().filter(|(_, before, after)| before != after).map(|(node, _, _)| *node).collect();
        reshaped.sort_unstable();
        reshaped.dedup();
        if reshaped.len() > MAX_PATCHED {
            return Self::calculate_all(workbook, functions);
        }
        if !reshaped.is_empty() {
            for node in &reshaped {
                self.remove_formula(*node);
            }
            let added = self.insert_formulas(workbook, functions, &reshaped);
            self.link_new(&added);
        }

        let cells: Vec<Node> = changed.iter().map(|(node, _, _)| *node).collect();
        // A cell set again with the same formula is a fresh cell, so it needs
        // telling whether the engine can run it.
        for &(sheet, addr) in &cells {
            let runnable = self.formulas.contains_key(&(sheet, addr));
            if let Some(cell) = workbook.sheets_mut()[sheet].get_mut(addr) {
                cell.uncalculated = cell.formula_text().is_some() && !runnable;
            }
        }
        let mut seeds = self.readers_of(&cells);
        seeds.extend(cells);
        seeds.extend(self.volatile.iter().copied());
        let affected = self.affected(seeds);
        self.compute(workbook, functions, affected);
        self
    }

    /// Parses the formulas at `nodes` and records what they read. Formulas the
    /// engine cannot run keep their stored value. Returns the runnable ones.
    /// Edges into them from other formulas are left to the caller.
    fn insert_formulas(&mut self, workbook: &mut Workbook, functions: &FunctionRegistry, nodes: &[Node]) -> Vec<Node> {
        let mut added = Vec::new();
        for &(sheet, addr) in nodes {
            let Some(cell) = workbook.sheets_mut()[sheet].get_mut(addr) else {
                continue;
            };
            let Some(text) = cell.formula_text() else {
                continue;
            };
            match runnable(text, functions) {
                Some((expr, is_volatile)) => {
                    cell.uncalculated = false;
                    if is_volatile {
                        self.volatile.insert((sheet, addr));
                    }
                    self.formulas.insert((sheet, addr), expr);
                    self.per_sheet[sheet].insert(addr);
                    added.push((sheet, addr));
                }
                None => cell.uncalculated = true,
            }
        }
        // Sources are found once every new formula is in place, so new formulas
        // reading each other are linked.
        for &node in &added {
            let reads = self.reads_of(workbook, node);
            let mut sources = Vec::new();
            for read in &reads {
                match *read {
                    Read::Cell(cell) => {
                        self.cell_readers.entry(cell).or_default().push(node);
                        if self.per_sheet[cell.0].contains(&cell.1) {
                            sources.push(cell);
                        }
                    }
                    Read::Range(sheet, range) => {
                        self.range_readers.push((sheet, range, node));
                        let inside =
                            self.per_sheet[sheet].range(range.start..=range.end).filter(|a| range.contains(**a));
                        sources.extend(inside.map(|addr| (sheet, *addr)));
                    }
                }
            }
            sources.sort_unstable();
            sources.dedup();
            self.reads.insert(node, reads);
            self.sources.insert(node, sources);
        }
        added
    }

    /// Adds the edges for formulas just inserted into an existing graph: from
    /// each to the formulas reading its cell, and from older formulas to it.
    fn link_new(&mut self, added: &[Node]) {
        let new: HashSet<Node> = added.iter().copied().collect();
        for &node in added {
            let mut readers: Vec<Node> =
                self.readers_of(&[node]).into_iter().filter(|reader| self.formulas.contains_key(reader)).collect();
            readers.sort_unstable();
            readers.dedup();
            for &reader in &readers {
                // A new reader found this one when its own sources were collected.
                if !new.contains(&reader) {
                    self.sources.entry(reader).or_default().push(node);
                }
            }
            self.dependents.insert(node, readers);
            for source in self.sources[&node].clone() {
                if !new.contains(&source) {
                    self.dependents.entry(source).or_default().push(node);
                }
            }
        }
    }

    /// Takes a formula out, with every edge to and from it.
    fn remove_formula(&mut self, node: Node) {
        if self.formulas.remove(&node).is_none() {
            return;
        }
        self.per_sheet[node.0].remove(&node.1);
        self.volatile.remove(&node);
        self.cycles.remove(&node);
        for source in self.sources.remove(&node).unwrap_or_default() {
            if let Some(list) = self.dependents.get_mut(&source) {
                list.retain(|other| *other != node);
            }
        }
        for dependent in self.dependents.remove(&node).unwrap_or_default() {
            if let Some(list) = self.sources.get_mut(&dependent) {
                list.retain(|other| *other != node);
            }
        }
        for read in self.reads.remove(&node).unwrap_or_default() {
            if let Read::Cell(cell) = read
                && let Some(list) = self.cell_readers.get_mut(&cell)
            {
                list.retain(|other| *other != node);
            }
        }
        if self.range_readers.iter().any(|(_, _, reader)| *reader == node) {
            self.range_readers.retain(|(_, _, reader)| *reader != node);
        }
    }

    /// Every cell and range a formula reads, with sheet names resolved.
    fn reads_of(&self, workbook: &Workbook, node: Node) -> Vec<Read> {
        let mut reads = Vec::new();
        self.formulas[&node].walk(&mut |part| {
            let resolve = |sheet: &Option<String>| match sheet {
                Some(name) => workbook.sheet_index(name),
                None => Some(node.0),
            };
            match part {
                Expr::Cell { sheet, cell } => {
                    if let Some(index) = resolve(sheet) {
                        reads.push(Read::Cell((index, cell.addr)));
                    }
                }
                Expr::Range { sheet, start, end } => {
                    if let Some(index) = resolve(sheet) {
                        reads.push(Read::Range(index, CellRange::new(start.addr, end.addr)));
                    }
                }
                _ => {}
            }
        });
        reads
    }

    /// Formulas that read any of `cells` directly.
    fn readers_of(&self, cells: &[Node]) -> Vec<Node> {
        let mut per_sheet: HashMap<usize, BTreeSet<CellAddr>> = HashMap::new();
        let mut readers = Vec::new();
        for cell in cells {
            readers.extend(self.cell_readers.get(cell).into_iter().flatten().copied());
            per_sheet.entry(cell.0).or_default().insert(cell.1);
        }
        for (sheet, range, reader) in &self.range_readers {
            let Some(cells) = per_sheet.get(sheet) else {
                continue;
            };
            if cells.range(range.start..=range.end).any(|addr| range.contains(*addr)) {
                readers.push(*reader);
            }
        }
        readers
    }

    /// The formulas among `seeds` and every formula depending on them.
    fn affected(&self, seeds: Vec<Node>) -> Vec<Node> {
        let mut seen: HashSet<Node> = HashSet::new();
        let mut queue: VecDeque<Node> = seeds.into_iter().filter(|node| self.formulas.contains_key(node)).collect();
        while let Some(node) = queue.pop_front() {
            if !seen.insert(node) {
                continue;
            }
            queue.extend(self.dependents.get(&node).into_iter().flatten().filter(|d| !seen.contains(*d)));
        }
        seen.into_iter().collect()
    }

    /// Computes `nodes`, each after the formulas it reads among them. Those that
    /// never become ready are in a cycle or depend on one.
    ///
    /// Formulas outside `nodes` keep their values. Every formula in a cycle with
    /// one of `nodes` is among them, since it depends on it, so a cycle outside
    /// stays as it was. Reading such a cycle still makes a formula part of it,
    /// so those sources count as never ready. This is iterative, so long chains
    /// cannot overflow the stack.
    fn compute(&mut self, workbook: &mut Workbook, functions: &FunctionRegistry, nodes: Vec<Node>) {
        let members: HashSet<Node> = nodes.iter().copied().collect();
        let blocks = |source: &Node| members.contains(source) || self.cycles.contains(source);
        let mut waiting_on: HashMap<Node, usize> = nodes
            .iter()
            .map(|node| (*node, self.sources[node].iter().filter(|source| blocks(source)).count()))
            .collect();
        let mut ready: VecDeque<Node> =
            waiting_on.iter().filter(|(_, count)| **count == 0).map(|(node, _)| *node).collect();
        while let Some(node) = ready.pop_front() {
            waiting_on.remove(&node);
            self.cycles.remove(&node);
            let evaluator = Evaluator { workbook, functions, sheet: node.0 };
            let expr = &self.formulas[&node];
            let value = evaluator.eval(expr);
            let format = if evaluator.is_date(expr) { NumberFormat::Date } else { NumberFormat::General };
            if let Some(cell) = workbook.sheets_mut()[node.0].get_mut(node.1) {
                cell.value = value;
                cell.format = format;
            }
            for dependent in self.dependents.get(&node).into_iter().flatten() {
                if let Some(count) = waiting_on.get_mut(dependent) {
                    *count -= 1;
                    if *count == 0 {
                        ready.push_back(*dependent);
                    }
                }
            }
        }
        for node in waiting_on.into_keys() {
            set_value(workbook, node, CellValue::Error(CellError::Cycle));
            self.cycles.insert(node);
        }
    }
}

fn set_value(workbook: &mut Workbook, (sheet, addr): Node, value: CellValue) {
    if let Some(cell) = workbook.sheets_mut()[sheet].get_mut(addr) {
        cell.value = value;
        cell.format = NumberFormat::General;
    }
}

/// Parses a formula the engine can run, and says whether it is volatile. A
/// formula with invalid syntax or an unknown function cannot run, so it keeps
/// its stored value, and formulas that read it use that value.
fn runnable(text: &str, functions: &FunctionRegistry) -> Option<(Expr, bool)> {
    let expr = parse(text).ok()?;
    let mut known = true;
    let mut volatile = false;
    expr.walk(&mut |part| {
        if let Expr::Call { name, .. } = part {
            match functions.get(name) {
                Some(function) => volatile |= function.volatile(),
                None => known = false,
            }
        }
    });
    known.then_some((expr, volatile))
}
