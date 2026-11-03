//! The right-click menu. What it offers follows where it opened: a cell, a
//! table's data or header, or a row or column header.
//!
//! Items that change the sheet become command lines or grid actions, so the
//! menu does exactly what typing or the shortcut does, undo included.

use egui::{Area, Button, Frame, Id, Key, Order, Pos2, RichText, Ui};
use excel_command::quote_arg;
use excel_model::{CellAddr, CellRange, MAX_COLS, MAX_ROWS, Sheet, col_name};

use crate::{FilterPopup, GridAction, GridState};

/// Where the menu was opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Place {
    Cell,
    RowHeader,
    ColHeader,
}

/// An open right-click menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ContextMenu {
    pub at: Pos2,
    pub place: Place,
    /// True in the frame the menu opened, whose click is not a click outside.
    pub just_opened: bool,
}

/// What choosing an item does.
#[derive(Debug, Clone, PartialEq)]
enum Choice {
    Action(GridAction),
    Command(String),
    /// Paste what the clipboard holds, as values only when true.
    Paste {
        values: bool,
    },
    OpenFilter(CellAddr),
    FitColumn(u32),
}

struct Item {
    label: String,
    shortcut: &'static str,
    choice: Choice,
}

struct Section {
    title: Option<String>,
    items: Vec<Item>,
}

fn item(label: impl Into<String>, shortcut: &'static str, choice: Choice) -> Item {
    Item { label: label.into(), shortcut, choice }
}

fn command(line: String) -> Choice {
    Choice::Command(line)
}

/// The menu's sections for where it opened. Items that cannot apply here are
/// left out rather than greyed, so the menu stays short.
fn sections(state: &GridState, sheet: &Sheet, place: Place) -> Vec<Section> {
    let selection = state.selection();
    let cursor = state.cursor;
    let mut sections = vec![Section {
        title: None,
        items: vec![
            item("Cut", "Ctrl+X", Choice::Action(GridAction::Cut(selection))),
            item("Copy", "Ctrl+C", Choice::Action(GridAction::Copy(selection))),
            item("Paste", "Ctrl+V", Choice::Paste { values: false }),
            item("Paste values", "", Choice::Paste { values: true }),
            item("Clear contents", "Delete", Choice::Action(GridAction::Clear(selection))),
        ],
    }];

    let (rows, cols) = (selection.rows(), selection.cols());
    let plural = |n: u32, one: &str, many: &str| if n == 1 { one.to_owned() } else { format!("{n} {many}") };
    let row_items = || {
        vec![
            item(
                format!("Insert {} above", plural(rows, "row", "rows")),
                "",
                command(format!("insert row {} {rows}", selection.start.row + 1)),
            ),
            item(
                format!("Delete {}", plural(rows, "row", "rows")),
                "",
                command(format!("delete row {} {rows}", selection.start.row + 1)),
            ),
        ]
    };
    let col_items = || {
        let at = col_name(selection.start.col);
        vec![
            item(
                format!("Insert {} left", plural(cols, "column", "columns")),
                "",
                command(format!("insert col {at} {cols}")),
            ),
            item(
                format!("Delete {}", plural(cols, "column", "columns")),
                "",
                command(format!("delete col {at} {cols}")),
            ),
        ]
    };

    match place {
        Place::RowHeader => sections.push(Section { title: Some("Rows".into()), items: row_items() }),
        Place::ColHeader => {
            let mut items = col_items();
            items.push(item("Fit width to contents", "Double-click edge", Choice::FitColumn(selection.start.col)));
            sections.push(Section { title: Some("Columns".into()), items });
        }
        Place::Cell => {
            if let Some(table) = sheet.table_at(cursor) {
                let header = CellAddr::new(table.header_row(), cursor.col);
                let name = sheet.display(header);
                let column = quote_arg(&name);
                let filtered = table.filters.iter().any(|filter| filter.column == cursor.col);
                let mut items = vec![
                    item(format!("Sort by {name}, smallest first"), "", command(format!("table sort {column}"))),
                    item(format!("Sort by {name}, largest first"), "", command(format!("table sort {column} desc"))),
                    item("Filter...", "", Choice::OpenFilter(header)),
                ];
                if !table.is_header(cursor) {
                    let value = sheet.display(cursor);
                    // The value goes twice so it is never read as a condition.
                    let values = format!("{} {}", quote_arg(&value), quote_arg(&value));
                    let shown = if value.is_empty() { "(Blanks)".to_owned() } else { value.clone() };
                    items.push(item(
                        format!("Filter by \"{shown}\""),
                        "",
                        command(format!("table filter {column} {values}")),
                    ));
                }
                if filtered {
                    items.push(item(
                        format!("Clear filter on {name}"),
                        "",
                        command(format!("table filter {column} clear")),
                    ));
                }
                items.push(item("Remove table, keep cells", "", command("table remove".into())));
                sections.push(Section { title: Some(format!("Table {}", table.name)), items });
            }
            let mut items = row_items();
            items.extend(col_items());
            sections.push(Section { title: Some("Rows and columns".into()), items });

            let mut items = Vec::new();
            if sheet.table_at(cursor).is_none() {
                let letter = col_name(cursor.col);
                items.push(item(
                    format!("Sort by column {letter}, smallest first"),
                    "",
                    command(format!("sort {letter}")),
                ));
                items.push(item(
                    format!("Sort by column {letter}, largest first"),
                    "",
                    command(format!("sort {letter} desc")),
                ));
                if !selection.is_single() {
                    items.push(item("Create table", "Ctrl+T", command("table create".into())));
                }
            }
            let plain = sheet.is_plain(cursor);
            let (label, style) =
                if plain { ("Show numbers with commas", "comma") } else { ("Show numbers without commas", "plain") };
            items.push(item(label, "", command(format!("format {selection} {style}"))));
            sections.push(Section { title: Some("Data".into()), items });

            let (some, all) = input_coverage(sheet, selection);
            let mut items = Vec::new();
            if !all {
                items.push(item("Mark as input cells", "", command(format!("app input {selection}"))));
            }
            if some {
                items.push(item("Unmark input cells", "", command(format!("app input clear {selection}"))));
            }
            sections.push(Section { title: Some("App".into()), items });
        }
    }
    sections
}

/// Whether some, and whether all, of `area` is input cells.
fn input_coverage(sheet: &Sheet, area: CellRange) -> (bool, bool) {
    let size = |r: CellRange| u64::from(r.rows()) * u64::from(r.cols());
    let covered: u64 = sheet
        .meta()
        .input_cells
        .overlapping(area)
        .map(|r| {
            let start = CellAddr::new(r.start.row.max(area.start.row), r.start.col.max(area.start.col));
            let end = CellAddr::new(r.end.row.min(area.end.row), r.end.col.min(area.end.col));
            size(CellRange::new(start, end))
        })
        .sum();
    (covered > 0, covered == size(area))
}

/// Shows the menu where it was opened. Choosing an item, `Esc`, or a click
/// elsewhere closes it.
pub(crate) fn show(ui: &Ui, state: &mut GridState, sheet: &Sheet, actions: &mut Vec<GridAction>) {
    let Some(menu) = state.menu.as_mut() else {
        return;
    };
    let just_opened = std::mem::take(&mut menu.just_opened);
    let (at, place) = (menu.at, menu.place);
    let sections = sections(state, sheet, place);
    let mut chosen = None;
    let area = Area::new(Id::unique("excel_grid_menu")).order(Order::Foreground).fixed_pos(at).constrain(true).show(
        ui.ctx(),
        |ui| {
            Frame::menu(ui.style()).show(ui, |ui| {
                ui.set_min_width(240.0);
                ui.spacing_mut().item_spacing.y = 1.0;
                for (index, section) in sections.iter().enumerate() {
                    if index > 0 {
                        ui.separator();
                    }
                    if let Some(title) = &section.title {
                        ui.label(RichText::new(title).small().weak());
                    }
                    for item in &section.items {
                        let button = Button::new(&item.label).shortcut_text(item.shortcut).frame(false);
                        if ui.add_sized([ui.available_width(), 22.0], button).clicked() {
                            chosen = Some(item.choice.clone());
                        }
                    }
                }
            })
        },
    );
    let escape = ui.input(|i| i.key_pressed(Key::Escape));
    let clicked_outside = !just_opened
        && ui.input(|i| i.pointer.any_pressed())
        && ui.input(|i| i.pointer.interact_pos()).is_some_and(|pos| !area.response.rect.contains(pos));
    if chosen.is_some() || escape || clicked_outside {
        state.menu = None;
    }
    match chosen {
        Some(Choice::Action(action)) => actions.push(action),
        Some(Choice::Command(line)) => actions.push(GridAction::RunCommand(line)),
        Some(Choice::Paste { values }) => {
            state.paste_values = values;
            actions.push(GridAction::RequestPaste);
        }
        Some(Choice::OpenFilter(header)) => state.popup = FilterPopup::open(sheet, header),
        Some(Choice::FitColumn(col)) => {
            actions.push(GridAction::ResizeColumn { col, width: crate::autofit::fitted_width(ui, sheet, col) });
        }
        None => {}
    }
}

/// Selects a whole row or column for a right click on its header, keeping a
/// selection of whole rows or columns that already holds it. The cursor goes
/// to the row's first cell or the column's top, and the view does not move.
pub(crate) fn select_header(state: &mut GridState, place: Place, index: u32) {
    let selection = state.selection();
    match place {
        Place::RowHeader => {
            let whole = selection.start.col == 0 && selection.end.col == MAX_COLS - 1;
            if !(whole && (selection.start.row..=selection.end.row).contains(&index)) {
                state.cursor = CellAddr::new(index, 0);
                state.anchor = CellAddr::new(index, MAX_COLS - 1);
            }
        }
        Place::ColHeader => {
            let whole = selection.start.row == 0 && selection.end.row == MAX_ROWS - 1;
            if !(whole && (selection.start.col..=selection.end.col).contains(&index)) {
                state.cursor = CellAddr::new(0, index);
                state.anchor = CellAddr::new(MAX_ROWS - 1, index);
            }
        }
        Place::Cell => {}
    }
}

#[cfg(test)]
mod tests {
    use excel_model::{Cell, SheetMeta, Table};

    use super::*;

    fn labels(state: &GridState, sheet: &Sheet, place: Place) -> Vec<String> {
        sections(state, sheet, place).into_iter().flat_map(|s| s.items).map(|i| i.label).collect()
    }

    fn table_sheet() -> Sheet {
        let mut sheet = Sheet::new("S");
        for (at, text) in [("A1", "지역"), ("A2", "서울"), ("A3", "")] {
            sheet.set(at.parse().unwrap(), Cell::parse_input(text));
        }
        sheet
            .set_meta(SheetMeta { tables: vec![Table::new("매출", "A1:A3".parse().unwrap())], ..Default::default() });
        sheet
    }

    #[test]
    fn a_plain_cell_offers_editing_rows_columns_and_data() {
        let mut state = GridState::default();
        let sheet = Sheet::new("S");
        let single = labels(&state, &sheet, Place::Cell);
        assert!(
            single.contains(&"Insert row above".to_owned())
                && single.contains(&"Sort by column A, smallest first".to_owned())
        );
        assert!(!single.contains(&"Create table".to_owned()), "one cell is not a range");
        state.anchor = "B3".parse().unwrap();
        let range = labels(&state, &sheet, Place::Cell);
        assert!(range.contains(&"Create table".to_owned()) && range.contains(&"Insert 3 rows above".to_owned()));
        assert!(range.contains(&"Delete 2 columns".to_owned()));
    }

    #[test]
    fn a_table_cell_offers_its_table() {
        let mut state = GridState::default();
        let sheet = table_sheet();
        state.cursor = "A2".parse().unwrap();
        state.anchor = state.cursor;
        let items = labels(&state, &sheet, Place::Cell);
        for wanted in ["Sort by 지역, smallest first", "Filter...", "Filter by \"서울\"", "Remove table, keep cells"]
        {
            assert!(items.contains(&wanted.to_owned()), "{wanted} in {items:?}");
        }
        assert!(!items.iter().any(|label| label.starts_with("Clear filter")), "no filter to clear yet");
        assert!(!items.contains(&"Create table".to_owned()));
        let header = {
            state.cursor = "A1".parse().unwrap();
            labels(&state, &sheet, Place::Cell)
        };
        assert!(!header.iter().any(|label| label.starts_with("Filter by")), "a header has no value to filter by");
    }

    #[test]
    fn headers_offer_rows_or_columns() {
        let state = GridState::default();
        let sheet = Sheet::new("S");
        let rows = labels(&state, &sheet, Place::RowHeader);
        assert!(rows.contains(&"Insert row above".to_owned()) && !rows.iter().any(|l| l.contains("column")));
        let cols = labels(&state, &sheet, Place::ColHeader);
        assert!(cols.contains(&"Fit width to contents".to_owned()) && cols.contains(&"Delete column".to_owned()));
    }

    #[test]
    fn items_become_the_right_lines() {
        let mut state = GridState::default();
        state.cursor = "B4".parse().unwrap();
        state.anchor = "B2".parse().unwrap();
        let sheet = Sheet::new("S");
        let lines: Vec<String> = sections(&state, &sheet, Place::Cell)
            .into_iter()
            .flat_map(|s| s.items)
            .filter_map(|i| match i.choice {
                Choice::Command(line) => Some(line),
                _ => None,
            })
            .collect();
        for wanted in
            ["insert row 2 3", "delete row 2 3", "insert col B 1", "sort B desc", "format B2:B4 plain", "table create"]
        {
            assert!(lines.contains(&wanted.to_owned()), "{wanted} in {lines:?}");
        }
    }

    #[test]
    fn input_cell_items_follow_the_selection() {
        let mut state = GridState::default();
        state.anchor = "B2".parse().unwrap();
        state.cursor = "B4".parse().unwrap();
        let mut sheet = Sheet::new("S");
        let app_lines = |state: &GridState, sheet: &Sheet| -> Vec<String> {
            sections(state, sheet, Place::Cell)
                .into_iter()
                .filter(|s| s.title.as_deref() == Some("App"))
                .flat_map(|s| s.items)
                .filter_map(|i| match i.choice {
                    Choice::Command(line) => Some(format!("{}: {line}", i.label)),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(app_lines(&state, &sheet), ["Mark as input cells: app input B2:B4"]);

        let mut meta = sheet.meta().clone();
        meta.input_cells.add("B3:C9".parse().unwrap());
        sheet.set_meta(meta);
        assert_eq!(
            app_lines(&state, &sheet),
            ["Mark as input cells: app input B2:B4", "Unmark input cells: app input clear B2:B4"],
            "partly input"
        );
        state.anchor = "B3".parse().unwrap();
        assert_eq!(app_lines(&state, &sheet), ["Unmark input cells: app input clear B3:B4"], "all input");
    }

    #[test]
    fn header_clicks_select_whole_rows_and_columns() {
        let mut state = GridState::default();
        select_header(&mut state, Place::RowHeader, 4);
        assert_eq!(state.selection().to_string(), "A5:XFD5");
        assert_eq!(state.cursor.col, 0);
        select_header(&mut state, Place::RowHeader, 4);
        assert_eq!(state.selection().to_string(), "A5:XFD5", "a second click keeps it");
        select_header(&mut state, Place::ColHeader, 2);
        assert_eq!(state.selection().to_string(), "C1:C1048576");
        assert_eq!(state.cursor.row, 0);
    }
}
