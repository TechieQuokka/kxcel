# kxcel: Design Document

Status: first version implemented, and every hand check in section 8 is done.
Version: 1.5.0. All crates share the workspace version. A new feature raises the
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
| Command line focus | `Ctrl+L` or a mouse click to enter, `Esc` to return to the grid | Typing in the grid starts cell editing, and `Ctrl+L` never conflicts with cell input |
| Theme | Dark by default, with light and dark switching added after the first version | Reviewed in the layout mockup. Switching is done by command |

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

### Formulas

- Arithmetic, cell references, and range references
- Absolute references such as `$A$1`
- Functions: `SUM`, `AVERAGE`, `COUNT`, `MIN`, `MAX`, `IF`
- Function completion: `=` lists every function, typing narrows the list, `Tab` or
  a click completes to `=SUM(`, arrows pick, and `Esc` closes the list
- Automatic recalculation when a value changes
- Visible errors, such as `#DIV/0!` and circular references
- Unsupported functions keep their original formula and stored value. The cell
  shows in the warning color, or as `?` with no stored value, and the status bar
  says it could not be calculated

### Command line

Press `Ctrl+L` or click to enter the command line, and `Esc` to return to the
grid. Commands autocomplete as they are typed.

| Command | Purpose |
|---|---|
| `w` | Save |
| `q` | Quit |
| `e` | Open a file |
| `sort` | Sort |
| `find` | Search |
| `goto` | Move to a cell |
| `insert` | Insert rows or columns |
| `delete` | Delete rows or columns |
| `sheet` | List, switch, or add sheets |
| `set` | Set a cell, so `kxcel-cli` can enter data |
| `help` | Show commands, functions, and keys, optionally filtered: `help sort` (1.3.0) |

A trailing `!` confirms an action that would lose something. `w!` saves over a
file with content the app cannot keep, and `q!` and `e!` discard unsaved changes.

`Ctrl+S` runs `w`.

### Files

- Read and write `.xlsx`, including multiple sheets
- Warn on open if a file has formatting, charts, or similar content, and refuse
  to overwrite it without `w!`, because saving rewrites the file and that content
  is lost. Saving to a new file with `w <file>` needs no confirmation
- A save replaces the file in one step, so a failure leaves the old file intact

### Excluded from the first version

- Decoration: fonts, colors, and borders
- Charts, merged cells, printing, and macros
- Filters and pivot tables

### Planned after the first version

**Theme switching**, using `theme dark` and `theme light`:

- Add `crates/commands/theme/` (`excel-cmd-theme`) and register it in
  `excel-features`, which also adds it to autocomplete.
- The command does not know about the GUI. It requests the change through an
  `Effect`, and `excel-app` applies it.
- `kxcel-cli` has no window, so it reports that the command is GUI-only.
- Colors live in one place from stage 2, with only the dark theme at first, so
  that adding a light theme does not require changing drawing code.

## 4. Usage

```sh
# GUI
kxcel data.xlsx

# Headless, using the same commands as the GUI command line
kxcel-cli data.xlsx -c "sort B" -c "w"
kxcel-cli data.xlsx --print
```

`kxcel-cli` processes files directly. It does not control a running window.
Commands run in order and stop at the first failure, and changes are written
only by `w`.

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

### Icon (1.2.0)

A pop-style icon of merged cells with a check mark, based on the "시안 7-2 – 팝
(병합 셀)" design reference. `assets/icon.svg` is the source. The PNG sizes were
rendered from it with headless Chrome, and the window icon is embedded as raw
RGBA so no image decoder is needed. On Wayland, the desktop shows the icon from
the desktop entry, so run `packaging/install.sh` to see it in the dock.

## 5. Architecture

### Project structure

There are 22 crates. Package names carry an `excel-` prefix because a crate
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
    │   │   └── src/ lib, address, range, value, cell, sheet, workbook
    │   ├── edit/                    # excel-edit
    │   │   └── src/ lib, edit, history, document
    │   └── command/                 # excel-command
    │       └── src/ lib, command, context, effect, error,
    │                services, registry, parse, testing
    │
    ├── formula/
    │   ├── syntax/                  # excel-formula-syntax
    │   │   ├── src/ lib, token, lexer, ast, parser, rewrite
    │   │   └── tests/ parser, rewrite
    │   ├── functions/               # excel-formula-functions
    │   │   ├── src/ lib, function, registry
    │   │   ├── src/math/ mod, sum, average, count, min, max
    │   │   ├── src/logic/ mod, if_fn
    │   │   └── tests/ math, logic
    │   └── engine/                  # excel-formula-engine
    │       ├── src/ lib, eval, deps, recalc, calculator
    │       └── tests/ arithmetic, references, errors, recalc
    │
    ├── io/
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
    │   ├── file/                    # excel-cmd-file: write, quit, open
    │   ├── sort/                    # excel-cmd-sort: sort
    │   ├── find/                    # excel-cmd-find: find
    │   ├── navigate/                # excel-cmd-navigate: goto
    │   ├── structure/               # excel-cmd-structure: insert, delete
    │   └── sheet/                   # excel-cmd-sheet: sheet
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
            └── src/ main, app, keymap, dialogs, actions, series, theme, fonts, icon, help
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
    fn help(&self) -> &str;
    fn run(&self, ctx: &mut Context, args: &[&str]) -> Result<(), CommandError>;
}
```

**Feature registration.** `excel-features` is the only place features are
listed, and both the GUI and the CLI use it. Autocomplete reads the same
registry, so a registered command appears as a candidate automatically.

**Service traits.** `w` needs file saving and `sort` needs recalculation, but
direct dependencies between features would break their independence. So
`excel-command` defines the `WorkbookStore` and `Calculator` traits, and the
implementations are supplied at runtime. Tests can supply fake implementations.

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

**Recalculation.** Every formula is recomputed after each edit, in dependency
order. The order is computed iteratively, so long chains cannot overflow the stack.

**Function trait.** The `Function` trait belongs to
`excel-formula-functions`, because only formulas use it. Each function has its
own file.

**Granularity limits.** Formula functions are split by file rather than by
crate, since one crate per function would make management harder. `excel-xlsx`
remains one crate because reading and writing implement the same contract.

### Adding a feature

- **Command:** add a folder under `crates/commands/`, add it to the root and
  `excel-features` manifests, then add one line to `excel-features`.
- **Formula function:** add one file under `excel-formula-functions`, then
  register it. Its help line appears in completion automatically.

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
| Large recalculation | 100,000 formulas stay within the 5-second test limit in a debug build. Each edit recomputes every formula, so very large files may feel slow | Make recalculation incremental if real use needs it |
| Column widths | Written to the file but not read back, so they reset on reopening | Add when real use needs it |
| Multi-line cells in Excel | Line breaks are saved, but Excel shows them on one line unless the cell has wrapping turned on, which this app does not write | Add a wrap format if files are shared with Excel |
| Dates | Read as serial numbers, with no date formatting | Add when real use needs it |
| Formula compatibility with Excel | Round trips are tested with `calamine` only. Files have not been opened in Excel itself | Check when a file is shared |

## 9. Environment

Checked on 2026-11-02.

- Ubuntu 24.04.5 LTS, Wayland session
- IBus input method
- `rustc` and `cargo` 1.99.1
