# kxcel: Design Document

Status: implemented through 1.19.0. Hand checks for 1.6.0 onward are listed in section 8.
Version: 1.19.0. All crates share the workspace version. A new feature raises the
minor version, such as 1.0.0 to 1.1.0, and a bug fix raises the patch version,
such as 1.1.0 to 1.1.1.
Last updated: 2026-11-02

## 1. Purpose

A compact spreadsheet for calculation on Ubuntu, written in Rust.

The window shows a cell grid and a command line. Cells behave like Excel, and
anything complex is done through commands instead of menus or buttons.

Features are added as real use reveals the need, so adding one must stay cheap.

### Principles

- **Calculation first.** Nothing is built purely for visual decoration.
- **Minimal interface.** No ribbon, menus, or toolbars.
- **Commands for complexity.** Complex operations belong on the command line.
- **Independent features.** Each feature can be used and tested on its own.

### Name and license

The app is called kxcel, and the binaries are `kxcel` and `kxcel-cli`. Excel is
a Microsoft trademark, so the name avoids it. Internal crate names keep their
`excel-` prefix, since users never see them.

The source is public under the PolyForm Noncommercial License 1.0.0: free for
personal use, research, and noncommercial organizations, with commercial use
requiring a paid license from the copyright holder. See `LICENSE` and
`COMMERCIAL.md`.

## 2. Decisions

| Topic | Decision | Reason |
|---|---|---|
| Language | Rust | Chosen by the project owner |
| Platform | Ubuntu | Intended environment |
| CLI and GUI | Command line inside the GUI window | Keyboard flow stays unbroken, and it is the simplest to build |
| File format | `.xlsx` read and write | Real Excel files can be opened and exchanged |
| Formula scope | Core formulas are calculated | Recalculation is essential, while hundreds of functions are excessive |
| Drag behavior | Fill handle and range selection | Both are core Excel calculation workflows |
| GUI library | `egui` with `eframe` | Mature, and released versions are usable directly. GPUI was considered and rejected |
| `.xlsx` libraries | `calamine` to read, `rust_xlsxwriter` to write | Values and formulas matter more here than preserving formatting |
| Code organization | Cargo workspace of fine-grained crates | Features stay independent within one repository |
| Standalone use | Library crates plus one shared headless CLI | The CLI reuses registered commands, so new features need no new binary |
| Prototype | Skipped, go directly to the project | Korean input is verified early in implementation instead |
| Formula display | Raw cell content appears in the status bar, with no separate formula bar | Keeps the window to cells and the command line |
| Command syntax | No `:` prefix, so a command is just `sort B` | The command line is a dedicated input, so no marker is needed |
| Command line focus | `Ctrl+L` or a mouse click to enter, `Esc` to return to the grid. After a command, the keyboard stays in the command line unless the command moved the cursor (1.6.0) | Typing in the grid starts cell editing, and `Ctrl+L` never conflicts with cell input. Staying lets commands run one after another |
| Command names | Whole words, no abbreviations: `save`, `open`, `exit` (1.6.0) | Easier to read and remember than `w`, `e`, and `q` |
| Theme | Dark by default, switched with `theme dark` or `theme light` and remembered (1.19.0) | Reviewed in the layout mockup. Switching is done by command |

## 3. Features

Features outside this first version are added when real use calls for them.

### Cells

- Cell selection, arrow key movement, and editing by typing or double-click
- Drag range selection, showing sum, average, and count
- Fill handle: dragging copies formulas and adjusts relative references.
  Dragging back inside the selection clears the cells left out, previewed in
  grey, and shrinks the selection (1.4.1)
- Fill series (1.5.0). Each line along the fill direction continues its own
  series, also backwards when filling up or left:
  - Numbers with a constant difference continue it: 1, 2, 3 gives 4, 5, 6
  - Otherwise, three or more nonzero numbers with a constant ratio continue
    it: 1, 2, 4 gives 8, 16
  - Text ending in a number counts: Item1 gives Item2, keeping zero padding
  - A single number copies, and Ctrl on release counts it up instead. Ctrl
    also copies a series instead of continuing it
  - Anything else repeats, with formula references adjusted. A value too
    large to represent is copied instead
  - Other patterns, such as Fibonacci, are left out on purpose: each added
    rule raises the chance of filling values nobody intended
- Dragging during formula entry inserts a range reference
- Copy, cut, paste, undo, and redo
- Column width adjustment and frozen row and column headers
- `Alt+Enter` adds a line break inside a cell. A row grows to show the most
  lines among its cells, and shrinks back when they are gone (1.4.0)
- Double-clicking a column edge in the header fits the column to its widest
  value, or restores the default width for an empty column (1.1.0)

### Dates (1.11.0)

- A date is a number, the serial day count Excel uses, shown as `YYYY-MM-DD`.
  Each cell has a format, `General` or `Date`, that only changes how its number
  is shown, so dates sort, add, and compare as numbers.
- Typing `2026-11-02`, `2026/11/2`, or `2026.11.02` enters a date. Editing a date
  shows it as a date again. Dates run from 1900-01-01 to 9999-12-31, numbered as
  Excel does, including its 1900-02-29.
- A formula shows a date when it gives one: a reference to a date, a date plus or
  minus days, or a function that returns a date. The days between two dates
  are a plain number.
- Filling continues dates as dates. A single date counts up a day at a time, as
  in Excel, and `Ctrl` copies it instead.
- `.xlsx` files keep dates both ways, and date-only cell styles no longer count
  as formatting that saving would lose. A date with a time of day shows as a
  number, and its time format still counts as lost.
- CSV files hold dates as `YYYY-MM-DD` text, which reads back as dates.
- Other number formats, such as decimal places, are left out on purpose.

### Formulas

- Arithmetic, cell references, and range references
- Absolute references such as `$A$1`
- Functions: `SUM`, `AVERAGE`, `COUNT`, `MIN`, `MAX`, `IF`
- Math and conditional functions (1.12.0): `ROUND`, `ROUNDUP`, `ROUNDDOWN`,
  `ABS`, `INT`, `MOD`, `POWER`, `SQRT`, `PRODUCT`, `COUNTA`, `COUNTBLANK`,
  `SUMIF`, `COUNTIF`, `AVERAGEIF`, `SUMIFS`, and `COUNTIFS`
  - Rounding is half away from zero after cutting to 15 significant digits, so
    `ROUND(2.675, 2)` is 2.68 as in Excel, though 2.675 is stored as 2.67499...
  - Conditions are a value, or text starting with `=`, `<>`, `<`, `<=`, `>`, or
    `>=`, such as `">5"`. Text matches ignoring case, with `*` and `?` as
    wildcards and `~` to escape them. `""` matches blank cells, and dates
    compare as their numbers
  - The ranges of `SUMIFS` and `COUNTIFS` must be the same size, or the result
    is `#VALUE!`
  - Functions receive a range as its shape plus only the cells holding values,
    so they can line up ranges without building every empty cell
- Logic and error functions (1.13.0): `AND`, `OR`, `NOT`, `IFERROR`, `IFS`,
  `ISBLANK`, `ISNUMBER`, `ISTEXT`, and `ISERROR`. In ranges, `AND` and `OR` count
  numbers and booleans and skip text and blanks, as Excel does. `IFS` with no
  true condition gives `#N/A`
- Text functions (1.14.0): `LEN`, `LEFT`, `RIGHT`, `MID`, `UPPER`, `LOWER`,
  `TRIM`, `CONCAT`, `TEXTJOIN`, `FIND`, `SUBSTITUTE`, `TEXT`, and `VALUE`
  - Positions and lengths count characters, so Korean text works as expected
  - `TEXT` covers common formats: `0` and `#` digits, `,` for thousands, `.` for
    decimals, `%`, quoted text, and dates with `yyyy`, `yy`, `m`, `mm`, `d`, `dd`.
    Formats with several sections, colors, or times are not supported
  - `VALUE` reads thousands separators, `%`, and dates
- Lookup functions (1.15.0): `VLOOKUP`, `HLOOKUP`, `INDEX`, `MATCH`, and `XLOOKUP`
  - As in Excel, `VLOOKUP`, `HLOOKUP`, and `MATCH` look for the nearest value in
    sorted data unless told to match exactly, and exact matches allow `*` and `?`
  - `XLOOKUP` matches exactly by default, needs no sorting, and takes a value to
    give when nothing is found. Its result range must line up with the searched
    one and be a single row or column
- Date functions (1.15.0): `TODAY`, `DATE`, `YEAR`, `MONTH`, `DAY`, and `DATEDIF`
  - `TODAY` uses the computer's time zone, read with `jiff`, and is recomputed
    after every edit
  - `DATE` carries over months and days past their end, as Excel does
- An empty argument, as in `XLOOKUP(a, b, c, , 2)`, is not supported. Give a
  value instead
- A test checks that every help example stating a result gives that result
- Whole-column references such as `A:A` are not supported yet. Use a range
  such as `A1:A1000`
- Function completion: typing a function name lists the functions starting with
  it, `Tab` or a click completes to `=SUM(`, arrows pick, and `Esc` closes the
  list. Since 1.15.2 the list opens from two letters, so `=` alone and a cell
  address such as `=A1` show nothing, and it shows at most eight functions,
  with a `+N more` line when more letters would narrow it
- `Tab` closes parentheses a formula leaves open (1.16.0), such as
  `=SUM(A1:A3` to `=SUM(A1:A3)`, and stays in the cell with a note in the
  command line, since where a parenthesis belongs can change the result. `Tab`
  again finishes the edit. Parentheses inside quoted text do not count, and
  with the function list open, `Tab` completes a function first. `Enter`, the
  arrows, and clicks enter the formula as typed, which shows `?` when it
  cannot be calculated
- Automatic recalculation when a value changes
- Visible errors, such as `#DIV/0!` and circular references
- Unsupported functions keep their original formula and stored value. The cell
  shows in the warning color, or as `?` with no stored value, and the status bar
  says it could not be calculated

### Command line

Press `Ctrl+L` or click to enter the command line, and `Esc` to return to the
grid. Commands autocomplete as they are typed.

After a command runs, the keyboard stays in the command line, so commands can
run one after another. A command that moves the cursor, such as `goto` or
`find`, returns to the grid instead. The rule follows the cursor, not the
command name, so new commands follow it too. A failed command stays with its
line put back, so it can be fixed and run again (1.6.0).

| Command | Purpose |
|---|---|
| `save` | Save, or save as another file with `save <file>` |
| `exit` | Exit the application |
| `open` | Open a file |
| `sort` | Sort |
| `find` | Search |
| `goto` | Move to a cell |
| `insert` | Insert rows or columns |
| `delete` | Delete rows or columns |
| `sheet` | List, switch, or add sheets |
| `fullscreen` | Full screen: `fullscreen on`, `fullscreen off`, or just `fullscreen` to switch, as `F11` does (1.18.0) |
| `theme` | Colors: `theme dark`, `theme light`, or just `theme` to switch. Remembered for the next start (1.19.0) |
| `set` | Set a cell, so `kxcel-cli` can enter data. It takes the rest of its line as typed, so quotes in formulas stay (1.15.1) |
| `help` | Show commands, functions, and keys, optionally filtered: `help sort` (1.3.0) |

A command that would lose something asks first (1.6.0). In the app, `exit` and
`open` with unsaved changes show the "Save / Don't save / Cancel" dialog, the
same one closing the window shows, and `save` over a file with content the app
cannot keep asks "Save anyway / Cancel". Saving first from the dialog asks that
second question too when it applies, then carries on.

A trailing `!` goes ahead without asking: `save!` saves over such a file, and
`exit!` and `open!` discard unsaved changes. `kxcel-cli` cannot show dialogs,
so scripts use these forms, and its errors name them.

Commands do not know about dialogs. A command that stops to ask returns a
`Confirmation` in its error, holding the command line that goes ahead, and the
app shows the dialog and runs that line once the user agrees.

`Ctrl+S` runs `save`. A document without a file name puts `save ` in the
command line to type one.

Paths complete with `Tab` after `open` and `save` (1.7.0), as in a terminal:
the list shows folders first, then files the app can read, matching the typed
start of the name and ignoring case. Hidden entries appear once a `.` is typed.
Picking a folder lists its contents next. A path with spaces is quoted, and
`~` stands for the home folder. A command offers paths by answering
`takes_path`, so new file commands get completion too.

`save <name>` without an extension adds `.xlsx`. A typed extension is kept.

### Files

- Read and write `.xlsx`, including multiple sheets
- Read and write `.csv` (1.8.0). Opening gives one sheet named after the file,
  with numbers, `TRUE`/`FALSE`, and error values recognized and the rest kept as
  text, including text starting with `=`. Saving writes the active sheet's
  values, so `save data.csv` asks first when formulas would keep only their
  values or other sheets would be left out. Files are UTF-8: a byte order mark
  is skipped on reading and none is written. The format is chosen by extension
- Warn on open if a file has formatting, charts, or similar content, and refuse
  to overwrite it without confirmation, because saving rewrites the file and that content
  is lost. Saving to a new file with `save <file>` needs no confirmation
- A save replaces the file in one step, so a failure leaves the old file intact

### Excluded from the first version

- Decoration: fonts, colors, and borders
- Charts, merged cells, printing, and macros
- Filters and pivot tables

### View (1.18.0, 1.19.0)

- **Full screen:** `fullscreen on`, `fullscreen off`, or `fullscreen` to switch,
  and `F11` the same. It follows the window manager too, if it leaves full
  screen itself. Full screen is not remembered.
- **Themes:** `theme dark`, `theme light`, or `theme` to switch. Dark is the
  default. Each theme is one palette in `theme.rs`, since widgets read colors
  from `egui::Visuals`, and function names in help get a darker purple on light.
- The choice of theme is kept in `~/.config/kxcel/settings.conf`, or under
  `$XDG_CONFIG_HOME`, as a line such as `theme = light`. A missing file or an
  unknown value gives the default, and tests never read or write the real file.
- Both commands live in `excel-cmd-view` and only ask the app through an
  `Effect`, since they know nothing about the window. `kxcel-cli` has no window,
  so it notes that they only apply in the app and carries on.

## 4. Usage

```sh
# GUI
kxcel data.xlsx

# Headless, using the same commands as the GUI command line
kxcel-cli data.xlsx -c "sort B" -c "save"
kxcel-cli data.xlsx --print
```

`kxcel-cli` processes files directly. It does not control a running window.
Commands run in order and stop at the first failure, and changes are written
only by `save`.

```
┌──────────────────────────────┐
│     A      B      C      D   │
│ 1  Name   Qty    Price  Total│
│ 2  Apple  3      1000   3000 │
│ 3  Pear   2      2000   4000 │
├──────────────────────────────┤
│ D3 =B3*C3  Sum 7000  Avg 3500│
│ sort B                       │
└──────────────────────────────┘
```

From top to bottom:

- **Grid:** takes most of the window, with row and column headers fixed.
- **Status bar:** current cell address, raw content, and selection statistics.
- **Command line:** at the bottom, with the completion popup opening above it.

The layout was reviewed and approved using the one-off mockup at
`test/index.html`. The real app is built with `egui`, so fonts and small visual
details may differ.

### Help (1.3.0)

`F1` or the `help` command opens a side panel on the right, listing commands,
functions, and keys with their usage. A search box narrows the list, and
`help sort` opens it already filtered. The grid stays usable while it is open.
Section titles say where each kind is typed, functions show as `=SUM(...)` in
their own color, and the completion lists are labelled Commands or Functions (1.4.2).
The lists come from the registries, so new commands and functions appear in help
automatically. `kxcel-cli --help` and `help` in the CLI print the same lists.

The panel was reworked in 1.9.0 to stay usable as commands and functions grow:

- Entries are grouped by category, such as File, Edit, and Navigate for
  commands, and Math, Logic, Text, Lookup, and Date for functions. Groups start
  folded, showing their size, and a search unfolds only the groups with matches.
- Picking an entry switches the panel to its page: summary, usage, description,
  arguments with optional ones marked, examples with results, and related
  entries that open their own pages. Back, or `Esc`, returns to the list.
- With the search box focused, the arrow keys move through the list and `Enter`
  opens the highlighted entry, or the only match.
- `help sum`, naming an entry exactly, opens its page. A partial name lists matches.
- `kxcel-cli` prints the same page for `-c "help sum"`, and lists by category otherwise.
- Two tabs since 1.17.0: **Functions**, typed in cells, and **Commands**, which
  also holds the keys as one group, since both work the application. Help opens
  on the last tab used, Functions at first. A search covers both tabs: each tab
  shows its number of matches, and when only the other tab has matches, help
  switches to it. Opening a page, from a search, a related link, or `help sum`,
  shows the tab it belongs to.

Each command and function describes itself with a `Doc` from `excel-model`,
which holds these parts. Only the summary and usage are required. The category
order is set once in `excel-features`, and a test checks that every entry has a
category and that related names exist. Command completion shows each command's
usage next to its summary.

### Icon (1.2.0)

A pop-style icon of merged cells with a check mark, based on the "시안 7-2 – 팝
(병합 셀)" design reference. `assets/icon.svg` is the source. The PNG sizes were
rendered from it with headless Chrome, and the window icon is embedded as raw
RGBA so no image decoder is needed. On Wayland, the desktop shows the icon from
the desktop entry, so run `packaging/install.sh` to see it in the dock.

## 5. Architecture

### Project structure

There are 24 crates. Package names carry an `excel-` prefix because a crate
named `core` could conflict with Rust's built-in `core` library.

```
excel/
├── Cargo.toml                       # members = ["crates/*/*"]
├── Cargo.lock
├── .gitignore
├── README.md
├── assets/
│   ├── icon.svg                     # app icon, the source for every size
│   └── icons/                       # PNG sizes for desktops, RGBA for the window
├── packaging/
│   ├── kxcel.desktop                # desktop entry
│   └── install.sh                   # installs binaries, entry, and icons per user
├── docs/
│   └── DESIGN.md
└── crates/
    ├── foundation/
    │   ├── model/                   # excel-model
    │   │   └── src/ lib, address, range, value, cell, date, doc, sheet, workbook
    │   ├── edit/                    # excel-edit
    │   │   └── src/ lib, edit, history, document
    │   └── command/                 # excel-command
    │       └── src/ lib, command, context, effect, error,
    │                services, registry, parse, path, testing
    │
    ├── formula/
    │   ├── syntax/                  # excel-formula-syntax
    │   │   ├── src/ lib, token, lexer, ast, parser, rewrite
    │   │   └── tests/ parser, rewrite
    │   ├── functions/               # excel-formula-functions
    │   │   ├── src/ lib, function, registry, criteria
    │   │   ├── src/math/ mod, sum, average, count, min, max, product, abs, int,
    │   │   │             mod_fn, power, sqrt, round, roundup, rounddown, counta,
    │   │   │             countblank, countif, countifs, sumif, sumifs, averageif
    │   │   ├── src/logic/ mod, if_fn, ifs, iferror, and, or, not,
    │   │   │              isblank, isnumber, istext, iserror
    │   │   ├── src/text/ mod, format, len, left, right, mid, upper, lower, trim,
    │   │   │             concat, textjoin, find, substitute, text_fn, value
    │   │   ├── src/lookup/ mod, vlookup, hlookup, index, match_fn, xlookup
    │   │   ├── src/date/ mod, today, date_fn, year, month, day, datedif
    │   │   └── tests/ math, logic
    │   └── engine/                  # excel-formula-engine
    │       ├── src/ lib, eval, graph, calculator
    │       └── tests/ arithmetic, references, errors, recalc, incremental, bench,
    │                  dates, math_functions, logic_functions, text_functions,
    │                  lookup_functions, date_functions, doc_examples
    │
    ├── io/
    │   ├── csv/                     # excel-csv
    │   │   └── src/ lib
    │   └── xlsx/                    # excel-xlsx
    │       ├── src/ lib, store, error
    │       ├── src/read/ mod, cells, formulas, sheets
    │       ├── src/write/ mod, cells, formulas, sheets
    │       ├── src/inspect/ mod, styles, charts
    │       └── tests/ roundtrip, inspect
    │
    ├── commands/                    # each: src/lib + command files with their tests
    │   ├── cell/                    # excel-cmd-cell: set
    │   ├── help/                    # excel-cmd-help: help
    │   ├── file/                    # excel-cmd-file: save, exit, open
    │   ├── sort/                    # excel-cmd-sort: sort
    │   ├── find/                    # excel-cmd-find: find
    │   ├── navigate/                # excel-cmd-navigate: goto
    │   ├── structure/               # excel-cmd-structure: insert, delete
    │   ├── sheet/                   # excel-cmd-sheet: sheet
    │   └── view/                    # excel-cmd-view: fullscreen, theme
    │
    ├── ui/
    │   ├── grid/                    # excel-ui-grid
    │   │   ├── src/ lib, state, layout, paint, headers, pointer, selection,
    │   │   │        fill, editor, clipboard, function_hint, autofit
    │   │   └── tests/ interaction
    │   ├── command-line/            # excel-ui-command-line
    │   │   └── src/ lib, state, input, completion_popup, message
    │   ├── status-bar/              # excel-ui-status-bar
    │   │   └── src/ lib, selection_stats
    │   └── help-panel/              # excel-ui-help-panel
    │       └── src/ lib, state, entries, panel
    │
    ├── assembly/
    │   └── features/                # excel-features
    │       └── src/ lib             # the one place features are registered
    │
    └── bin/
        ├── cli/                     # excel-cli, binary: kxcel-cli
        │   ├── src/ main, args, runner
        │   └── tests/ cli
        └── app/                     # excel-app, binary: kxcel
            └── src/ main, app, keymap, dialogs, actions, series, theme, settings,
                     fonts, icon, help
```

### Dependency rules

| Layer | May depend on |
|---|---|
| `foundation` | Lower foundation crates only (`model` <- `edit` <- `command`) |
| `formula`, `io`, `commands` | `foundation` and crates in the same group. Commands never depend on each other |
| `ui` | `foundation` and `egui` only |
| `assembly/features` | `foundation`, `formula`, `io`, `commands` |
| `bin` | `foundation` and `assembly`. Only `app` may also use `ui` |

### Key mechanisms

**Command trait.** Every command implements one contract:

```rust
pub trait Command {
    fn name(&self) -> &str;
    fn doc(&self) -> Doc;
    fn run(&self, ctx: &mut Context, args: &[&str]) -> Result<(), CommandError>;
    fn takes_path(&self) -> bool { false }
}
```

**Feature registration.** `excel-features` is the only place features are
listed, and both the GUI and the CLI use it. Autocomplete reads the same
registry, so a registered command appears as a candidate automatically.

**Service traits.** `save` needs file saving and `sort` needs recalculation, but
direct dependencies between features would break their independence. So
`excel-command` defines the `WorkbookStore` and `Calculator` traits, and the
implementations are supplied at runtime. Tests can supply fake implementations.
`excel-features` supplies a `FileStore` that hands each path to `excel-csv` or
`excel-xlsx` by extension, and `save_losses` lets a format say what saving to it
would leave out.

**Korean input.** Focus is requested only when missing, because `egui` interrupts composition on every request. A text box stays focused at the cursor even when no edit is in
progress, at one pixel wide. The system only offers input methods to a focused
text box, so this lets the first key of a Korean word start composing. Enter,
Tab, and arrows are read after the text box runs, because an input method can
deliver a commit and a key in the same frame.

**Scrolling.** The grid scrolls by whole rows and columns and only measures
visible cells, so positions stay precise on a sheet of 1,048,576 rows.

**Row heights.** Heights are not stored. Each visible row is measured from the
most lines among its cells, so multi-line cells from any file show correctly.
Scrolling to a far cell measures only the rows near it.

**Clipboard.** Values holding a tab, a line break, or a quote are quoted, as
Excel does, so multi-line cells copy and paste between applications.

**Recalculation.** Incremental since 1.10.0. The engine keeps a dependency
graph of the workbook it last calculated: what each formula reads, by single
references and by ranges, and the edges between formulas. Every change to cells
reaches it through `Calculator::update` with the edit, including undo and redo.

- A value edit finds the formulas reading the changed cells and follows the
  edges from there, so only those are computed.
- A formula edit patches just that formula's edges, then computes what it affects.
  More than 256 formulas changed at once rebuild the graph instead.
- The affected formulas are sorted among themselves, iteratively so long chains
  cannot overflow the stack. Those that never become ready are in a cycle or
  read one, and show `#CYCLE!`.
- Volatile functions, such as `TODAY`, are recomputed after every edit.
- Changes to whole sheets, such as sort, insert, delete, and adding a sheet, and
  opening a file recalculate everything. So does a workbook the graph was not
  built for: each workbook, including each copy, has its own identity.

A test replays over a thousand random histories of edits, undos, and redos
across two sheets, with ranges, cross-sheet references, cycles, and unknown
functions, and checks after every step that the result equals recalculating
from scratch. Timings, from `cargo test --release -p excel-formula-engine --test
bench -- --ignored --nocapture`, on 200,000 formulas:

| Edit | Before (1.9.0) | Incremental |
|---|---|---|
| Value read by one formula and one total | 480 ms | 6 ms |
| Value at the top of a 100,000-step chain | 450 ms | 125 ms |
| Value no formula reads | 620 ms | 0.004 ms |
| Formula in the middle of the chain | 450 ms | 65 ms |

**Function trait.** The `Function` trait belongs to
`excel-formula-functions`, because only formulas use it. Each function has its
own file, grouped in folders by category. Besides its name, `Doc`, and `call`, a
function may say it is `volatile`, recomputed after every edit like `TODAY`,
and that it `returns_date`, so its formula shows a date.

**Function arguments.** A reference reaches a function as `Cells`: the shape of
the range plus only the cells holding values, with their positions. Functions
that line ranges up, such as `SUMIF` and `VLOOKUP`, read positions from it, and
a large range costs only what it holds. Conditions such as `">5"` are shared
by the conditional functions in `criteria`.

**Granularity limits.** Formula functions are split by file rather than by
crate, since one crate per function would make management harder. `excel-xlsx`
remains one crate because reading and writing implement the same contract.

### Adding a feature

- **Command:** add a folder under `crates/commands/`, add it to the root and
  `excel-features` manifests, then add one line to `excel-features`.
- **Formula function:** add one file under `excel-formula-functions`, then
  register it. Its `Doc` appears in completion and help automatically.

### Independent testing

Every crate that holds logic has its own tests and can be tested alone:

```sh
cargo test -p excel-xlsx
```

Non-GUI crates do not depend on `egui`, so they can be tested without a window.
The grid and the app are also tested without a window, by feeding simulated input
into `egui`. `excel_command::testing` provides fake services for command crates.

## 6. Dependencies

Versions were checked on 2026-11-02. Check crates.io for the latest version
again when each dependency is added.

| Crate | Version | Purpose |
|---|---|---|
| `eframe`, `egui` | 0.37.0 | GUI |
| `calamine` | 0.38.0 | Read `.xlsx` |
| `rust_xlsxwriter` | 0.102.0 | Write `.xlsx` |
| `zip` | 9.2.2 | Inspect `.xlsx` contents for what saving would lose |
| `csv` | 1.4.0 | Read and write `.csv` (1.8.0) |
| `jiff` | 0.2.38 | Today's date in the local time zone for `TODAY` (1.15.0) |
| `tempfile` | 3.27.0 | Temporary files in tests |

Shared dependency versions are managed once, in the workspace root.

### Rejected alternatives

- **GPUI:** relatively new, but the newest official release found on crates.io
  was 0.2.2 from October 2025, and it is still in development. Its performance
  benefit for this application is uncertain.
- **iced:** clean architecture, but long compile times are a common complaint.
- **Slint:** its strengths are hard to use on a custom-drawn cell grid.
- **`umya-spreadsheet`:** a single library for reading and writing, but its
  large-file performance and preservation behavior are unverified.
- **Separate repositories:** changing a shared contract would mean editing every
  repository.
- **Runtime plugins:** the plugin boundary is too complex for a compact tool.
- **One executable per feature:** adds a binary and argument handling for every
  feature.

## 7. Implementation Plan

All stages are implemented. Each was verified with automated tests as it was
built. The hand checks from stage 2 are listed in section 8.

| Stage | Scope | Done when |
|---|---|---|
| 1. Skeleton | Workspace, all crates, docs, Git setup | Full build and test pass |
| 2. Grid and input | Large grid, cell movement and editing, command line, centralized colors | Korean input and scrolling confirmed by hand on Ubuntu |
| 3. Formulas | Parser, core functions, recalculation, error display | Formula crates pass their own tests |
| 4. Files | `.xlsx` read and write, formatting and chart warnings | Formula and computed value round-trip tests pass |
| 5. Commands | Registry, command crates, `kxcel-cli`, autocomplete | CLI processes a real file |
| 6. Drag and convenience | Selection stats, fill handle, clipboard, undo, sheet switching | Real usage flow confirmed |

## 8. Risks and Open Items

| Item | Status | Next step |
|---|---|---|
| Korean input | Confirmed by hand on Ubuntu 24.04 with Wayland and IBus, in cells | Done |
| Scrolling and drag | Confirmed by hand on a 200,000-row file, including drag selection, column resizing, and fitting | Done |
| App icon | Confirmed by hand in the GNOME dock and app list after `packaging/install.sh` | Done |
| Help panel and multi-line cells | Confirmed by hand: `F1`, search, `Alt+Enter`, and taller rows | Done |
| Fill shrink, command and function split, fill series | Confirmed by hand in 1.5.0: geometric fill, shrinking, Ctrl to count up, and the help split | Done |
| Window close prompt and Korean commands | Confirmed by hand: the unsaved-changes prompt, Cancel, Don't save, and `find` with Korean | Done |
| Command names, confirmation dialogs, and focus (1.6.0) | Covered by automated tests | Check by hand: `exit`/`open` dialogs, `save` over a formatted file, staying in the command line |
| Path completion (1.7.0) | Covered by automated tests | Check by hand: `Tab` through folders, Korean names, names with spaces |
| CSV (1.8.0) | Covered by automated tests | Check a file from Excel or another tool |
| Help panel rework (1.9.0) | Covered by automated tests | Check by hand: folding groups, pages, related links, arrow keys |
| Dates (1.11.0) | Covered by automated tests | Check a file with dates from Excel |
| Functions (1.12.0 to 1.15.0) | Covered by automated tests, including every help example with a result | Compare a few results with Excel |
| Completion limits, Tab closing parentheses, help tabs (1.15.2 to 1.17.0) | Covered by automated tests | Check by hand while typing formulas |
| Full screen and themes (1.18.0, 1.19.0) | Covered by automated tests, which cannot see the real window | Check `F11`, `theme light`, and that the theme returns after a restart |
| Large recalculation | Incremental since 1.10.0, and checked against full recalculation by a randomized test. Opening a file and whole-sheet commands still compute everything | Done |
| Column widths | Written to the file but not read back, so they reset on reopening | Add when real use needs it |
| Multi-line cells in Excel | Line breaks are saved, but Excel shows them on one line unless the cell has wrapping turned on, which this app does not write | Add a wrap format if files are shared with Excel |
| Dates | Shown as `YYYY-MM-DD` since 1.11.0. Times of day are not shown | Add times if real use needs them |
| Formula compatibility with Excel | Round trips are tested with `calamine` only. Files have not been opened in Excel itself | Check when a file is shared |

## 9. Environment

Checked on 2026-11-02.

- Ubuntu 24.04.5 LTS, Wayland session
- IBus input method
- `rustc` and `cargo` 1.99.1
