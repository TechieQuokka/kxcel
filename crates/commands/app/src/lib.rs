//! The `app` command: marks input cells for app mode, and registers
//! workbooks as apps in the desktop's app list.

mod app;
mod install;

use excel_command::CommandRegistry;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(app::AppCommand);
}
