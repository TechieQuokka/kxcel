//! Command contract, registry, parsing, and service traits.

mod command;
mod context;
mod effect;
mod error;
mod parse;
pub mod path;
mod registry;
mod services;
pub mod testing;

pub use command::Command;
pub use context::Context;
pub use effect::Effect;
pub use error::{CommandError, Confirmation};
pub use excel_model::Doc;
pub use parse::{quote_arg, split_line};
pub use registry::{Candidate, CommandRegistry};
pub use services::{Calculator, LoadedWorkbook, StoreError, Structural, WorkbookStore};
