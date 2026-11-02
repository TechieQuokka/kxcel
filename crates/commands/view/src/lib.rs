//! Commands for how the window looks. They only ask the host through an
//! [`excel_command::Effect`], since they know nothing about the window, and
//! `kxcel-cli`, which has none, says so.

mod fullscreen;
mod switch;
mod theme;

use excel_command::CommandRegistry;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(fullscreen::Fullscreen);
    registry.register(theme::ThemeCommand);
}
