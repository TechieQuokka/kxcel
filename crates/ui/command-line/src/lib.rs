//! Command line widget with completion.

mod completion_popup;
mod input;
mod message;
mod state;

pub use input::{CommandLineAction, show};
pub use state::CommandLineState;
