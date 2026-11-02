//! Workbook, sheet, cell, and address types.
//!
//! This crate knows nothing about formulas beyond storing their text, nothing
//! about file formats, and nothing about the GUI.

mod address;
mod cell;
mod date;
mod doc;
mod range;
mod sheet;
mod value;
mod workbook;

pub use address::{CellAddr, MAX_COLS, MAX_ROWS, col_from_name, col_name};
pub use cell::{Cell, CellInput, NumberFormat};
pub use date::{MAX_DATE_SERIAL, date_serial, format_date, parse_date, serial_date};
pub use doc::Doc;
pub use range::CellRange;
pub use sheet::{DEFAULT_COL_WIDTH, Sheet};
pub use value::{CellError, CellValue, format_number};
pub use workbook::Workbook;
