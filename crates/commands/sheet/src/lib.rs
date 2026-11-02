//! Sheet switching command.

mod sheet;

use excel_command::CommandRegistry;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(sheet::SheetCommand);
}
