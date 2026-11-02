//! Read and write `.xlsx` files.

mod error;
mod inspect;
mod read;
mod store;
mod write;

pub use inspect::inspect;
pub use read::read;
pub use store::XlsxStore;
pub use write::write;
