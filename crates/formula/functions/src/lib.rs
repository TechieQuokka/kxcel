//! Built-in spreadsheet functions.
//!
//! To add a function, add one file with a [`Function`] implementation and
//! register it in [`FunctionRegistry::with_builtins`].

mod function;
mod logic;
mod math;
mod registry;

pub use function::{Arg, Function, to_bool, to_number};
pub use registry::FunctionRegistry;
