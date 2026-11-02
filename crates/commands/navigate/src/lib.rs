//! Go-to command.

mod goto;

use excel_command::CommandRegistry;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(goto::Goto);
}
