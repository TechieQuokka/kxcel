//! Help side panel listing commands, functions, and keys.
//!
//! The panel knows nothing about where entries come from. The application
//! builds them from its registries, so new commands and functions appear here
//! without changes to this crate.

mod entries;
mod panel;
mod state;

pub use entries::{HelpEntry, HelpSection, filter};
pub use panel::{HelpAction, show};
pub use state::HelpState;
