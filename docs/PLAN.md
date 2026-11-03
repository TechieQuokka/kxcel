# kxcel: Work Plan for 1.21.0 to 1.29.0

Status: planned on 2026-11-02, revised on 2026-11-03 (hidden rows in
`Ctrl+Arrow`, a light edit for sheet metadata), and implemented through
1.29.0 on 2026-11-03, with the whole plan approved at once. `docs/DESIGN.md`
describes what was built. Where the build differs from this plan: hidden rows
are kept as runs (`RowSet`) rather than a `BTreeSet`, so long hidden runs are
cheap to pass over; table growth lives in `Document::apply`, so every way of
setting cells follows it; tables, filters, and hidden rows are read from the
file's XML, since `calamine` does not say where a table is; and `Ctrl+T`
always uses `TableN`. The patch for filters in `.xlsx` proved reliable, so
the fallback was not needed.
Base version: 1.20.0 (commit `5bc338d`).

This plan covers three features agreed in discussion: jumping with
`Ctrl+Arrow`, tables with sort and filter, and app mode. Each stage below is
one version. A new feature raises the minor version, as `docs/DESIGN.md`
describes, and every stage ends with all tests passing, `cargo clippy` clean,
`cargo fmt` applied, and `docs/DESIGN.md` updated. Commits, pushes, and
installs happen only when the owner asks.

## Overview

| Version | Stage | Size |
|---|---|---|
| 1.21.0 | `Ctrl+Arrow` and `Ctrl+Shift+Arrow` | Small |
| 1.22.0 | Tables: model, create, remove, resize, look, growth | Large |
| 1.23.0 | Table sort | Small |
| 1.24.0 | Hidden rows and table filters | Large |
| 1.25.0 | Header `▼` popup for sort and filter | Medium |
| 1.26.0 | Tables, filters, and hidden rows in `.xlsx` | Large |
| 1.27.0 | Input cells for app mode | Medium |
| 1.28.0 | App mode window | Large |
| 1.29.0 | Registering apps with text icons | Medium |

The order follows dependencies. `Ctrl+Arrow` gives the data-edge rule that
table creation reuses. Tables exist before sorting and filtering them, hidden
rows exist before the popup shows them, and everything is in the model before
it is saved. App mode needs input cells before its window, and its window
before it can be registered.

---

## 1.21.0: `Ctrl+Arrow` and `Ctrl+Shift+Arrow`

### Behavior

- `Ctrl+Arrow` moves the cursor as Excel does:
  - From a filled cell whose neighbor in that direction is filled, to the last
    filled cell of that run.
  - Otherwise, to the next filled cell in that direction.
  - With nothing further, to the sheet edge: row 1,048,576, column XFD, row 1,
    or column A.
- `Ctrl+Shift+Arrow` moves the same way and extends the selection from the
  anchor, so the status bar shows its sum, average, and count.
- Only while not editing a cell. During an edit, arrows behave as today.
- The view scrolls to the new cursor, as `goto` does.

### Design

- A pure function in `excel-ui-grid`, such as
  `data_edge(sheet, from, direction) -> CellAddr`, so it can be tested without
  a window and reused by table creation.
- "Filled" means a cell exists with a value that is not empty. A formula giving
  empty text counts as filled, as in Excel.
- Speed: moving along a row uses the sheet's ordered cell map directly. Moving
  along a column checks cells row by row, but skips gaps by asking the map for
  the next filled row in that column, so a sparse 1,048,576-row sheet stays
  fast. Target: under 50 ms for any jump in a debug build.

### Files

- `crates/ui/grid/src/` new `edge.rs`, keys in `editor.rs` (`keys_while_ready`)
- `crates/foundation/model/src/sheet.rs` if a helper for the next filled cell
  in a column is needed
- `crates/bin/app/src/help.rs` two key entries

### Tests

- Unit tests for `data_edge`: runs, gaps, a single cell, empty sheet, every
  direction, and the four sheet edges.
- Grid interaction tests for `Ctrl+Arrow`, `Ctrl+Shift+Arrow`, and that an edit
  in progress is not affected.
- A timing test on a large sparse sheet, like the existing large-sheet tests.

---

## 1.22.0: Tables: model, create, remove, resize, look, growth

### Behavior

- `table create [name]`, and `Ctrl+T` in the window:
  - With several cells selected, the selection becomes the table.
  - With one cell selected, the table is the data region around it, found by
    growing from the cell while the next row or column holds data, using the
    1.21.0 edge rule.
  - The first row is always the header. A header cell that is empty or repeats
    another gets a name such as `Column3`, as Excel does.
  - The name defaults to `Table1`, `Table2`, and so on. Names are unique in the
    workbook, ignoring case, and follow Excel's rules: a letter or underscore
    first, then letters, digits, underscores, or periods, no spaces, and not a
    cell address such as `A1`.
  - Tables cannot overlap.
- `table remove`: the table under the cursor stops being a table. Its cells
  stay as they are.
- `table resize <range>`: sets the table's range. The header row stays the
  first row, and the range must still contain it.
- `table`, with nothing after it, lists the tables on the sheet.
- Growth: entering a value in the row just below the table, inside its columns,
  adds that row. Entering a value in the header row just right of the table adds
  a column, with that value as its header.
- Look: the header row gets a light background and bold text, and a thin
  border marks the table's outline. Both come from the theme palette, so dark
  and light themes each have their own shades.
- Every change is undoable, growth included, and undo restores both the cells
  and the table in one step.

### Design

- Model, in `excel-model`:
  - `Table { name: String, range: CellRange, filters: Vec<ColumnFilter>,
    sorted: Option<(u32, bool)> }`, where `range` includes the header row and
    `sorted` remembers the last sort for the `▼` mark only.
  - `SheetMeta.tables: Vec<Table>`, with helpers on `Sheet`: `table_at(addr)`,
    `table_by_name(name)`, and header lookup by name, ignoring case.
  - Row and column insert and delete move and resize tables, as they move
    formulas today.
- Sheet metadata: tables, and later hidden rows (1.24.0) and input cells
  (1.27.0), live in one `SheetMeta` struct held by `Sheet`, with
  `Sheet::meta()` and `Sheet::set_meta()`. `Sheet`'s hand-written `PartialEq`
  must compare it, or undo tests would miss metadata changes.
- Edits: growth has to change cells and a table in one undo step. Add two
  `Edit` variants:
  - `Edit::SetMeta { sheet: usize, before: SheetMeta, after: SheetMeta }`, for
    changes to tables, filters, hidden rows, and input cells that leave cells
    alone.
  - `Edit::Batch(Vec<Edit>)`, applied in order and reverted in reverse.
  Growth becomes a batch of the cell change and a `SetMeta`, recorded as one
  history entry.
- Recalculation: `FormulaEngine::update` today recalculates the whole workbook
  for anything that is not `SetCells`. Using `Edit::ReplaceSheets` for table
  changes would throw away the incremental recalculation from 1.15.x on every
  growth, filter, or input-cell change. So the engine skips `SetMeta`, since
  formulas do not read metadata (structured references are out of scope), and
  for a `Batch` of only `SetCells` and `SetMeta` it updates incrementally with
  the cell changes gathered in order. Any other batch recalculates fully.
  Changes that move cells, such as table sort and row insert, keep using
  `ReplaceSheets` as today.
- Growth is decided where cells are committed: `actions::commit` in the app and
  `set` through `Context::apply`. To keep one rule, add a helper in
  `excel-edit` or a new small crate function `grow_tables(sheet, changes)` that
  both call.
- Commands: a new crate `crates/commands/table/` (`excel-cmd-table`), with
  `table` taking subcommands, registered in `excel-features`, with a new
  command category `Table` in `COMMAND_CATEGORIES` and a `Doc` for help.
- Drawing: `crates/ui/grid/src/paint.rs` draws the header background and
  outline after cells and before the selection.

### Tests

- Model: create, overlap refused, names, `table_at`, header lookup, and that
  inserting and deleting rows and columns move tables.
- Region detection from one cell, including a header with gaps.
- Commands: create, remove, resize, list, errors, and undo.
- Growth: below and right, not diagonally, not outside the columns, and one
  undo step.
- Edits: `SetMeta` and `Batch` apply and revert correctly, and the formula
  engine stays incremental for them, as `crates/formula/engine/tests/incremental.rs`
  checks for `SetCells`.
- App test: `Ctrl+T` creates a table at the cursor.

### Risks

- Growth must not trigger on paste or fill that only touches the row below by
  accident. Rule: growth applies to any committed change that sets a value in
  the row just below the table within its columns, the same as Excel, and undo
  takes it back.

---

## 1.23.0: Table sort

### Behavior

- `table sort <column> [desc]`: sorts the data rows of the table under the
  cursor by the column with that header, ignoring case. The header row stays.
- Same order as `sort`: numbers, then text, booleans, and errors, with empty
  cells last either way. Formulas move with their rows.
- One time only: the table does not keep itself sorted. It remembers the last
  column and direction only to show a small arrow on that header.
- Hidden rows, once filters exist, are sorted together with the visible ones,
  and filters are applied again after sorting, so what shows is still what
  matches.
- Undoable.

### Design

- Reuse the value ordering in `crates/commands/sort/src/sort.rs`. Commands must
  not depend on each other, so move that ordering to `excel-model`, as a
  `CellValue` ordering function such as `sort_order`, and have both `sort` and
  `table sort` use it.

### Tests

- Sort by header name, both directions, empty cells last, formulas moved with
  adjusted references, unknown column, cursor outside any table, undo.

---

## 1.24.0: Hidden rows and table filters

### Behavior

- `table filter <column> <values...>`: keeps rows whose cell in that column
  equals one of the values.
- `table filter <column> "<condition>"`: keeps rows meeting a condition with the
  `COUNTIF` syntax: `>5000`, `<>서울`, `사과*`, `""` for blanks.
- A filter replaces any earlier filter on the same column. Filters on several
  columns combine with AND.
- `table filter <column> clear` removes one column's filter, and
  `table filter clear` removes all filters of the table.
- Rows that do not match are hidden. Hidden rows stay hidden until filters are
  applied again, by any `table filter` or `table sort`, even if their values
  change, as in Excel.
- While a sheet has hidden rows, row numbers show in the accent color and skip
  hidden rows, and the status bar shows `12 of 340 rows` for the table under the
  cursor.
- Formulas count hidden rows. The status bar's sum, average, and count, and
  `Ctrl+C`, use visible rows only.
- The cursor and arrows skip hidden rows. `goto` to a hidden cell shows the
  cursor at the next visible row and says the row is hidden.
- `Ctrl+Arrow` and `Ctrl+Shift+Arrow` skip hidden rows, as in Excel: a hidden
  cell neither continues nor ends a run, and an up or down jump never lands on
  a hidden row. With every row below hidden, the jump stops at the last visible
  row.
- Undoable, through `Edit::SetMeta`, without recalculating formulas.

### Design

- Model: `SheetMeta.hidden_rows: BTreeSet<u32>`, set by applying filters. Kept
  explicitly rather than derived, so edits do not make rows vanish while typing.
  `ColumnFilter { column: u32, rule: FilterRule }`, where
  `FilterRule::Values(Vec<String>)` or `FilterRule::Condition(String)`.
- Conditions reuse the `Criterion` in `excel-formula-functions`. It is crate
  private today, so make it public, or move it to `excel-model`, which both the
  functions crate and the table command can reach. Moving it is cleaner, since
  commands should not depend on the formula crates.
- Grid: `layout.rs` and `state.rs` count rows through a visible-row mapping
  that skips hidden rows. Scrolling already works in whole rows, so it walks
  visible rows instead of row numbers. Row heights stay measured per visible
  row. Paint draws row numbers from the real row.
- Status bar: `selection_stats.rs` skips hidden rows. Clipboard copy skips
  hidden rows. Fill and paste act on every row, as today.
- `data_edge` from 1.21.0 takes the hidden rows into account. Moving along a
  column, it treats hidden rows as absent; moving along a row, nothing changes,
  since the cursor is never on a hidden row. Skipping uses the ordered set, so
  the 50 ms target from 1.21.0 still holds with many hidden rows.
- Rows inserted or deleted move hidden rows, as they move tables.

### Tests

- Filter by values, by condition, on two columns, clear one, clear all.
- Hidden rows stay hidden after an edit and update after filtering again.
- Grid: layout skips hidden rows, arrows skip them, row numbers are real,
  scrolling a large filtered sheet stays fast.
- `data_edge` with hidden rows: a run broken only by hidden rows, a filled cell
  that is hidden, every row below hidden, and the timing test with many hidden
  rows.
- Status bar stats and copy use visible rows only, while `=SUM` counts all.
- Undo restores filters and hidden rows together.

### Risks

- The grid assumes rows are contiguous in several places, such as scrolling,
  drag selection, and the fill handle. Each must be checked. This is the largest
  change in the plan, so it gets its own stage before the popup.

---

## 1.25.0: Header `▼` popup for sort and filter

### Behavior

- Each table header cell shows a small `▼` at its right edge. It shows the sort
  arrow after `table sort`, and a filter mark while the column has a filter.
- Clicking `▼` opens a popup under the header cell with:
  - Sort ascending and Sort descending.
  - A condition box, using the `COUNTIF` syntax.
  - A search box and a checkbox list of the column's distinct values, with
    Select all. Values show as they display, dates as dates.
  - Apply, Clear filter, and Cancel.
- Every choice runs the matching `table` command through the command registry,
  so the popup and the command line behave the same and undo works the same.
- `Esc` closes the popup. Clicking outside closes it without changes.

### Design

- A new module in `excel-ui-grid` for the popup, returning a `GridAction` such
  as `RunCommand(String)` that the app runs with `run_lines`, so the grid still
  knows nothing about commands.
- The distinct value list is built from the column's data rows, capped at a few
  thousand entries with a note when more exist.

### Tests

- Clicking `▼` opens the popup for that column. Choosing sort or filter emits
  the expected command line. The app runs it and the result matches typing the
  command.

---

## 1.26.0: Tables, filters, and hidden rows in `.xlsx`

### Behavior

- Saving writes each table in Excel's table format, with its name, range,
  header row, and filter buttons, writes hidden rows as hidden, and writes each
  column filter, so Excel shows the same table and filter.
- Opening reads tables, their filters, and hidden rows from files made by
  kxcel or by Excel.
- Files with tables no longer count tables as something saving would lose.

### Design

- Writing: `rust_xlsxwriter` 0.102.0 writes tables (`Worksheet::add_table`,
  `Table::set_name`, `set_header_row`, `set_autofilter`, `set_columns`) and
  hidden rows (`set_row_hidden`). It does not write filter conditions inside a
  table. Plan: after saving, open the saved file with the `zip` crate, add
  `filterColumn` elements to each table part's `autoFilter`, and write the file
  again, inside the same one-step replace that saving already uses.
- Reading: `calamine` 0.38.0 reads table names, ranges, and data
  (`load_tables`, `table_names_in_sheet`, `table_by_name`). Filter conditions and
  hidden rows are read from the sheet and table XML through the `zip` crate, as
  `inspect` already reads styles.
- Value filters map to Excel's `filters` element, and conditions to
  `customFilters`. Conditions Excel cannot express keep their hidden rows but
  are written as the closest form, with a warning in the message.
- Check crates.io for newer versions of `rust_xlsxwriter` and `calamine` first,
  in case table filters became supported, which would remove the patch step.

### Tests

- Round trip of a table with a header, values, two filters, and hidden rows.
- A file made with a table by another writer opens with its table.
- The patched file opens again with `calamine`, and the `inspect` warnings no
  longer list tables.

### Risks

- Patching XML after `rust_xlsxwriter` is the most fragile part of the plan. If
  it proves unreliable, the fallback is to save tables and hidden rows only, so
  Excel shows the same rows without filter buttons set, and to discuss it with
  the owner before choosing.

---

## 1.27.0: Input cells for app mode

### Behavior

- `app input <range>` marks cells as input cells. `app input clear` unmarks every
  input cell on the sheet, and `app input clear <range>` unmarks some.
- `app input`, with nothing after it, lists the input ranges.
- In normal mode, input cells show a light tint, so it is clear what an app will
  let users change. Editing is not restricted in normal mode.
- Saved as Excel's unlocked cell protection, so Excel sees the same cells as
  unlocked. Opening a file reads it back.
- Undoable.

### Design

- Model: a per-cell flag would not cover empty input cells, so keep
  `SheetMeta.input_cells: Vec<CellRange>`, merged and split as ranges are added
  and cleared. Row and column insert and delete move them. Changes go through
  `Edit::SetMeta`, so marking cells does not recalculate formulas.
- Writing: an unlocked `rust_xlsxwriter::Format` (`Format::set_unlocked`) on each
  input cell, including empty ones, combined with the date format where a cell
  is a date.
- Reading: from `xl/styles.xml`, the cell styles whose `protection` has
  `locked="0"`, and the cells using them, as `inspect/styles.rs` already maps
  styles. The style check that warns about lost formatting must treat
  unlocked-only styles as kept, as it treats date-only styles.
- Commands in a new crate `crates/commands/app/` (`excel-cmd-app`), with an
  `App` command category.

### Tests

- Mark, list, clear part, clear all, undo, and moves on insert and delete.
- Round trip through `.xlsx`, including empty input cells and dates.
- A file with unlocked cells from another writer gives the same input cells and
  no formatting warning.

---

## 1.28.0: App mode window

### Behavior

- `excel --app <file>` opens the file in app mode, and `--nosave` makes it an
  app that never saves.
- The window shows only the grid, without the command line or row and column
  headers. The status bar stays, showing the current cell's value but never a
  formula.
- Only input cells can be edited. Typing elsewhere does nothing.
- `Tab` and `Enter` move to the next input cell, left to right, then down, and
  `Shift` moves back. Clicking any cell selects it so it can be read.
- Input cells keep the tint from normal mode, in both themes.
- Saving:
  - Default: `Ctrl+S` saves, and closing with changes asks Save, Don't save, or
    Cancel, as in normal mode.
  - `--nosave`: nothing saves and closing never asks. Each start is fresh.
- No commands: `Ctrl+L`, `F1`, and command shortcuts do nothing. `F11` and the
  theme still apply.
- Window title: the app's name when given, otherwise the file name.

### Design

- `crates/bin/app/src/main.rs` reads `--app`, `--nosave`, `--name`, and `--id`.
  The `--id` sets the window's app id, so Wayland matches the window to the
  app's own desktop entry and shows its icon in the dock.
- `ExcelApp` gets a mode. In app mode it skips the command line and help panels
  and passes a read-only rule to the grid, which refuses to start an edit
  outside input cells and moves between input cells on `Tab` and `Enter`.
- The grid hides headers through a flag in its layout.

### Tests

- App mode refuses edits outside input cells, accepts them inside, and moves
  between input cells in order.
- `Ctrl+L` and `F1` do nothing. `--nosave` never saves and never asks.
- The default saves with `Ctrl+S` and asks on close.

---

## 1.29.0: Registering apps with text icons

### Behavior

- `app install <name> [nosave] [icon=<text>]` registers the open file as an app
  named `<name>` in the app list and dock. The file must have been saved.
- The icon shows the first two characters of the name, such as `견적` for
  `견적 계산기` or `Bu` for `Budget`, on a rounded square whose color comes from
  the name, so different apps get different colors. `icon=견` sets the
  characters.
- `app uninstall <name>` removes the registration. The file stays.
- `app list` lists registered apps.
- Works from `kxcel-cli` too, since it only writes files.

### Design

- Each app gets an id from a short hash of its name, such as
  `kxcel-app-3f9a1c2e`, since desktop file names must be plain ASCII and names
  may be Korean.
- Files written for the current user:
  - `~/.local/share/applications/<id>.desktop`, with `Name=<name>`,
    `Exec=<this program> --app --id <id> --name "<name>" [--nosave] "<file>"`,
    `Icon=<id>`, and `StartupWMClass=<id>`. The program path comes from the
    running executable, so it works whether installed as `excel` or `kxcel`.
  - `~/.local/share/icons/hicolor/scalable/apps/<id>.svg`, the text icon.
  - Then `update-desktop-database` and `gtk-update-icon-cache`, as
    `packaging/install.sh` runs, ignoring their absence.
- The SVG uses a generic sans-serif font family, which on Ubuntu resolves to a
  font with Korean glyphs. Colors come from a fixed set of pleasant hues picked
  by the hash, with white text, readable at 16 to 512 pixels.
- Registered apps are found by their desktop files, so `app list` and
  `app uninstall` need no other record.

### Tests

- With a temporary data folder in place of `~/.local/share`: install writes the
  desktop file and icon with the expected contents, uninstall removes them,
  list shows them, and names with spaces, Korean, and quotes work.
- The icon text rule: two characters by default, `icon=` overrides, and the
  color is the same for the same name.
- Hand check: the app appears in the app list and dock with its icon, opens in
  app mode, and `nosave` behaves.

---

## Help and Documentation Across Stages

- New commands get a `Doc` with category, usage, description, arguments, and
  examples, so help and completion show them. Categories `Table` and `App` are
  added to `COMMAND_CATEGORIES`.
- New keys go into the Keys group of help: `Ctrl+Arrow`, `Ctrl+Shift+Arrow`,
  `Ctrl+T`.
- `docs/DESIGN.md` gains sections for tables and app mode, the command table
  gains `table` and `app`, the structure tree gains the new crates, and the hand
  check table in section 8 gains one row per stage.
- `README.md` gains a line each for tables and app mode.

## Out of Scope, Agreed

- Structured references such as `=SUM(Table1[가격])`.
- Keeping a table sorted after it changes.
- A `SUBTOTAL` function for sums of visible rows.
- Autosave in app mode.
- Image icons for apps.

## Open Questions for Implementation

These were not discussed and are left for the owner when the stage starts:

- 1.22.0: whether `Ctrl+T` asks for a name or always uses `TableN`. The plan
  uses `TableN`, renamed later with a future command if wanted.
- 1.26.0: the fallback if patching table filters into `.xlsx` proves unreliable.
