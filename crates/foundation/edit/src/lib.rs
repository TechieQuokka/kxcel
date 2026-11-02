//! Reversible edits, undo history, and the open document.

mod document;
mod edit;
mod history;

pub use document::Document;
pub use edit::{CellChange, Edit};
pub use history::History;
