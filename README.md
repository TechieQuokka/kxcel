# kxcel

A compact spreadsheet for calculation on Ubuntu, written in Rust.

The window shows a cell grid and a command line. Cells behave like a familiar
spreadsheet, and anything complex is done through commands.

```sh
kxcel data.xlsx
kxcel-cli data.xlsx -c "sort B" -c "w"
```

In the window, press `F1` for help, `Ctrl+L` for the command line, where
commands autocomplete with `Tab`, and `Esc` to return to the grid. In a cell,
type `=` to list functions.

## Features

- `.xlsx` reading and writing, with a warning before saving over content the
  app cannot keep
- Formulas with automatic recalculation: arithmetic, references, `SUM`,
  `AVERAGE`, `COUNT`, `MIN`, `MAX`, `IF`
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
