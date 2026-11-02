//! Sort command.

mod sort;

use excel_command::CommandRegistry;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(sort::Sort);
}
