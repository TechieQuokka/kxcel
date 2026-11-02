//! Built-in spreadsheet functions.
//!
//! To add a function, add one file with a [`Function`] implementation and
//! register it in [`FunctionRegistry::with_builtins`].

mod criteria;
mod date;
mod function;
mod logic;
mod lookup;
mod math;
mod registry;
mod text;

pub use function::{Arg, Cells, Function, scalar, scalar_number, to_bool, to_number};
pub use registry::FunctionRegistry;
