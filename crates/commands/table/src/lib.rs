//! The `table` command: create, remove, resize, and list tables.

mod table;

use excel_command::CommandRegistry;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(table::TableCommand);
}
