//! Reversible edits, undo history, and the open document.

mod document;
mod edit;
mod growth;
mod history;

pub use document::Document;
pub use edit::{CellChange, Edit};
pub use growth::{add_table_row, with_table_growth};
pub use history::History;
