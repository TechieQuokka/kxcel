//! Workbook, sheet, cell, and address types.
//!
//! This crate knows nothing about formulas beyond storing their text, nothing
//! about file formats, and nothing about the GUI.

mod address;
mod cell;
mod criterion;
mod date;
mod doc;
mod range;
mod ranges;
mod rows;
mod sheet;
mod sort;
mod table;
mod value;
mod workbook;

pub use address::{CellAddr, MAX_COLS, MAX_ROWS, col_from_name, col_name};
pub use cell::{Cell, CellInput, NumberFormat};
pub use criterion::Criterion;
pub use date::{MAX_DATE_SERIAL, date_serial, format_date, parse_date, serial_date};
pub use doc::Doc;
pub use range::CellRange;
pub use ranges::RangeSet;
pub use rows::RowSet;
pub use sheet::{DEFAULT_COL_WIDTH, Sheet};
pub use sort::sort_order;
pub use table::{ColumnFilter, FilterRule, SheetMeta, Table, header_names, table_name_problem};
pub use value::{CellError, CellValue, format_number};
pub use workbook::Workbook;
