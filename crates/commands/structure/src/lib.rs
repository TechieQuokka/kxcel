//! Insert and delete commands.

mod delete;
mod insert;
mod target;

use excel_command::CommandRegistry;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(insert::Insert);
    registry.register(delete::Delete);
}
