//! The popup under a table header's `▼`: sort, a condition, and a list of the
//! column's values to tick.
//!
//! Every choice becomes a `table` command line, so the popup does exactly what
//! typing that command does, undo included.

use egui::{Align2, Area, Frame, Id, Key, Order, Pos2, RichText, ScrollArea, TextEdit, Ui};
use excel_command::quote_arg;
use excel_model::{CellAddr, CellValue, FilterRule, Sheet, sort_order};

use crate::GridAction;

/// The most values the list shows, so a huge column stays quick to open.
pub const MAX_VALUES: usize = 5000;

/// An open popup for one table column.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterPopup {
    /// The header cell the popup belongs to.
    pub header: CellAddr,
    /// The column's header text, as `table` commands name the column.
    pub name: String,
    /// Each distinct value as it shows, with whether it is ticked. An empty
    /// string stands for blank cells.
    pub values: Vec<(String, bool)>,
    /// How many distinct values were left out of the list.
    pub more: usize,
    pub search: String,
    pub condition: String,
    /// Whether the column has a filter now, so Clear filter means something.
    pub filtered: bool,
    /// True in the frame the popup opened, whose click is not a click outside.
    just_opened: bool,
}

impl FilterPopup {
    /// Builds the popup for the header cell `header`, or `None` if it is not
    /// a table header. Values are ticked as the current filter shows them.
    pub fn open(sheet: &Sheet, header: CellAddr) -> Option<Self> {
        let table = sheet.table_at(header).filter(|table| table.is_header(header))?;
        let rule = table.filters.iter().find(|filter| filter.column == header.col).map(|filter| &filter.rule);

        let mut found: Vec<(CellValue, String)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for row in table.data_rows() {
            let addr = CellAddr::new(row, header.col);
            let shown = sheet.display(addr);
            if seen.insert(shown.to_lowercase()) {
                found.push((sheet.value(addr).clone(), shown));
            }
        }
        found.sort_by(|a, b| sort_order(&a.0, &b.0, false));
        let more = found.len().saturating_sub(MAX_VALUES);
        found.truncate(MAX_VALUES);

        let ticked = |shown: &str| match rule {
            Some(FilterRule::Values(values)) => {
                values.iter().any(|value| value.trim().eq_ignore_ascii_case(shown.trim()))
            }
            _ => true,
        };
        let values = found.into_iter().map(|(_, shown)| (shown.clone(), ticked(&shown))).collect();
        let condition = match rule {
            Some(FilterRule::Condition(condition)) => condition.clone(),
            _ => String::new(),
        };
        Some(Self {
            header,
            name: sheet.display(header),
            values,
            more,
            search: String::new(),
            condition,
            filtered: rule.is_some(),
            just_opened: true,
        })
    }

    pub fn sort_line(&self, descending: bool) -> String {
        let direction = if descending { " desc" } else { "" };
        format!("table sort {}{direction}", quote_arg(&self.name))
    }

    pub fn clear_line(&self) -> String {
        format!("table filter {} clear", quote_arg(&self.name))
    }

    /// The command Apply runs: the condition if one is typed, else the ticked
    /// values. `None` when nothing is ticked, or when everything is and there
    /// is no filter to clear.
    pub fn apply_line(&self) -> Option<String> {
        let column = quote_arg(&self.name);
        let condition = self.condition.trim();
        if !condition.is_empty() {
            return Some(format!("table filter {column} {}", quote_arg(condition)));
        }
        let ticked: Vec<&str> = self.values.iter().filter(|(_, on)| *on).map(|(value, _)| value.as_str()).collect();
        if ticked.is_empty() {
            return None;
        }
        if ticked.len() == self.values.len() && self.more == 0 {
            return self.filtered.then(|| self.clear_line());
        }
        let mut args: Vec<String> = ticked.iter().map(|value| quote_arg(value)).collect();
        if let [one] = &args[..] {
            // One argument could read as a condition, such as a value holding
            // `*`, so the value goes twice, which still means just that value.
            args.push(one.clone());
        }
        Some(format!("table filter {column} {}", args.join(" ")))
    }

    /// Values matching the search box, with their places in the list.
    fn shown_values(&self) -> Vec<usize> {
        let search = self.search.trim().to_lowercase();
        (0..self.values.len())
            .filter(|i| search.is_empty() || self.values[*i].0.to_lowercase().contains(&search))
            .collect()
    }
}

/// What the user did in the popup this frame.
enum Choice {
    Run(String),
    Close,
}

/// Shows the popup below its header. Returns a command to run, and closes the
/// popup when the user chose something, pressed `Esc`, or clicked elsewhere.
pub(crate) fn show(ui: &Ui, popup: &mut Option<FilterPopup>, anchor: Pos2, actions: &mut Vec<GridAction>) {
    let Some(state) = popup.as_mut() else {
        return;
    };
    let mut choice = None;
    let area = Area::new(Id::unique("excel_filter_popup"))
        .order(Order::Foreground)
        .fixed_pos(anchor)
        .pivot(Align2::LEFT_TOP)
        .show(ui.ctx(), |ui| {
            Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(260.0);
                choice = contents(ui, state);
            })
        });
    let escape = ui.input(|i| i.key_pressed(Key::Escape));
    let just_opened = std::mem::take(&mut state.just_opened);
    let clicked_outside = !just_opened
        && ui.input(|i| i.pointer.any_pressed())
        && ui.input(|i| i.pointer.interact_pos()).is_some_and(|pos| !area.response.rect.contains(pos));
    if escape || clicked_outside {
        choice = choice.or(Some(Choice::Close));
    }
    match choice {
        Some(Choice::Run(line)) => {
            actions.push(GridAction::RunCommand(line));
            *popup = None;
        }
        Some(Choice::Close) => *popup = None,
        None => {}
    }
}

fn contents(ui: &mut Ui, state: &mut FilterPopup) -> Option<Choice> {
    let mut choice = None;
    ui.label(RichText::new(&state.name).strong());
    ui.horizontal(|ui| {
        if ui.button("Sort ascending").clicked() {
            choice = Some(Choice::Run(state.sort_line(false)));
        }
        if ui.button("Sort descending").clicked() {
            choice = Some(Choice::Run(state.sort_line(true)));
        }
    });
    ui.separator();
    ui.label(RichText::new("Condition, as for COUNTIF: >5000, <>서울, 사과*").small().weak());
    ui.add(TextEdit::singleline(&mut state.condition).hint_text("Condition").desired_width(f32::INFINITY));
    ui.separator();
    ui.add(TextEdit::singleline(&mut state.search).hint_text("Search values").desired_width(f32::INFINITY));

    let shown = state.shown_values();
    let all = !shown.is_empty() && shown.iter().all(|i| state.values[*i].1);
    let mut select_all = all;
    if ui.checkbox(&mut select_all, "Select all").changed() {
        for i in &shown {
            state.values[*i].1 = select_all;
        }
    }
    ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
        for i in shown {
            let (value, on) = &mut state.values[i];
            let label = if value.is_empty() { "(Blanks)".to_owned() } else { value.clone() };
            ui.checkbox(on, label);
        }
    });
    if state.more > 0 {
        ui.label(RichText::new(format!("{} more values not listed", state.more)).small().weak());
    }
    ui.separator();
    ui.horizontal(|ui| {
        let apply = state.apply_line();
        if ui.add_enabled(apply.is_some(), egui::Button::new("Apply")).clicked() {
            choice = apply.map(Choice::Run);
        }
        if ui.add_enabled(state.filtered, egui::Button::new("Clear filter")).clicked() {
            choice = Some(Choice::Run(state.clear_line()));
        }
        if ui.button("Cancel").clicked() {
            choice = Some(Choice::Close);
        }
    });
    choice
}

#[cfg(test)]
mod tests {
    use excel_model::{Cell, ColumnFilter, SheetMeta, Table};

    use super::*;

    fn addr(text: &str) -> CellAddr {
        text.parse().unwrap()
    }

    fn sheet(filter: Option<FilterRule>) -> Sheet {
        let mut sheet = Sheet::new("S");
        for (at, text) in
            [("A1", "지역 이름"), ("A2", "서울"), ("A3", "부산"), ("A4", "서울"), ("A6", "10"), ("A7", "9")]
        {
            sheet.set(addr(at), Cell::parse_input(text));
        }
        let mut table = Table::new("T", "A1:A7".parse().unwrap());
        table.filters.extend(filter.map(|rule| ColumnFilter { column: 0, rule }));
        sheet.set_meta(SheetMeta { tables: vec![table], ..Default::default() });
        sheet
    }

    #[test]
    fn opens_only_on_table_headers_with_distinct_sorted_values() {
        let popup = FilterPopup::open(&sheet(None), addr("A1")).unwrap();
        let values: Vec<&str> = popup.values.iter().map(|(v, _)| v.as_str()).collect();
        assert_eq!(values, ["9", "10", "부산", "서울", ""], "numbers by size, then text, blanks last");
        assert!(popup.values.iter().all(|(_, on)| *on));
        assert!(FilterPopup::open(&sheet(None), addr("A2")).is_none());
        assert!(FilterPopup::open(&sheet(None), addr("B1")).is_none());
    }

    #[test]
    fn makes_command_lines() {
        let mut popup = FilterPopup::open(&sheet(None), addr("A1")).unwrap();
        assert_eq!(popup.sort_line(false), "table sort \"지역 이름\"");
        assert_eq!(popup.sort_line(true), "table sort \"지역 이름\" desc");
        assert_eq!(popup.apply_line(), None, "everything ticked and no filter to clear");
        popup.values[3].1 = false;
        assert_eq!(popup.apply_line().unwrap(), "table filter \"지역 이름\" 9 10 부산 \"\"");
        for value in &mut popup.values {
            value.1 = value.0 == "부산";
        }
        assert_eq!(popup.apply_line().unwrap(), "table filter \"지역 이름\" 부산 부산", "one value goes twice");
        popup.values.iter_mut().for_each(|value| value.1 = false);
        assert_eq!(popup.apply_line(), None);
        popup.condition = " >5 ".into();
        assert_eq!(popup.apply_line().unwrap(), "table filter \"지역 이름\" >5");
    }

    #[test]
    fn reflects_the_current_filter() {
        let popup = FilterPopup::open(&sheet(Some(FilterRule::Values(vec!["서울".into()]))), addr("A1")).unwrap();
        let ticked: Vec<&str> = popup.values.iter().filter(|(_, on)| *on).map(|(v, _)| v.as_str()).collect();
        assert_eq!(ticked, ["서울"]);
        assert_eq!(popup.clear_line(), "table filter \"지역 이름\" clear");
        let popup = FilterPopup::open(&sheet(Some(FilterRule::Condition(">5".into()))), addr("A1")).unwrap();
        assert_eq!(popup.condition, ">5");
        assert!(popup.filtered);
        let mut all = FilterPopup::open(&sheet(Some(FilterRule::Values(vec!["서울".into()]))), addr("A1")).unwrap();
        all.values.iter_mut().for_each(|value| value.1 = true);
        assert_eq!(all.apply_line().unwrap(), all.clear_line(), "ticking everything clears the filter");
    }

    #[test]
    fn search_narrows_the_list() {
        let mut popup = FilterPopup::open(&sheet(None), addr("A1")).unwrap();
        popup.search = "서".into();
        assert_eq!(popup.shown_values(), [3]);
    }
}
