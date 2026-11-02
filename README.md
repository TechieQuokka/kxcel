# kxcel

A compact spreadsheet for calculation on Ubuntu, written in Rust.

The window shows a cell grid and a command line. Cells behave like a familiar
spreadsheet, and anything complex is done through commands.

```sh
kxcel data.xlsx
kxcel-cli data.xlsx -c "sort B" -c "save"
```

In the window, press `F1` for help, `Ctrl+L` for the command line, where
commands autocomplete with `Tab`, and `Esc` to return to the grid. In a cell,
type `=` to list functions.

## Features

- `.xlsx` reading and writing, with a warning before saving over content the
  app cannot keep
- Formulas with automatic, incremental recalculation and 55 functions: math
  and conditional sums (`SUM`, `ROUND`, `SUMIF`, `COUNTIFS`), logic (`IF`, `IFS`,
  `IFERROR`, `AND`), text (`LEFT`, `TEXT`, `TEXTJOIN`, `SUBSTITUTE`), lookup
  (`VLOOKUP`, `XLOOKUP`, `INDEX`, `MATCH`), and dates (`TODAY`, `DATE`, `DATEDIF`)
- Dates shown as `YYYY-MM-DD`, typed the same way
- `.csv` reading and writing
- Commands such as `save`, `open`, `exit`, `sort`, `find`, and `goto`, with
  `Tab` completing command names and file paths
- A help panel with a page for every command and function (`F1`)
- Fill handle that continues series such as 1, 2, 3 and 1, 2, 4
- Multi-line cells, Korean input, undo, copy and paste
- A headless CLI that runs the same commands as the app

## Build and install

```sh
cargo test --workspace
packaging/install.sh    # installs kxcel and kxcel-cli for the current user
```

See [docs/DESIGN.md](docs/DESIGN.md) for design decisions and architecture.

## License

kxcel is **free for personal use, research, and education**, under the
[PolyForm Noncommercial License 1.0.0](LICENSE).

**Commercial use, including any use by a company, requires a paid license.**
See [COMMERCIAL.md](COMMERCIAL.md).

kxcel is not affiliated with Microsoft. Excel is a trademark of Microsoft
Corporation.
