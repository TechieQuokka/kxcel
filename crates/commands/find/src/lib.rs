//! Find command.

mod find;

use excel_command::CommandRegistry;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(find::Find);
}
