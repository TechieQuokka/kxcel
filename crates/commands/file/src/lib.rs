//! Save, quit, and open commands.
//!
//! A trailing `!` confirms an action that would lose something: `w!` saves over
//! a file with formatting the application cannot keep, and `q!` and `e!`
//! discard unsaved changes.

mod open;
mod quit;
mod write;

use excel_command::CommandRegistry;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(write::Write { force: false });
    registry.register(write::Write { force: true });
    registry.register(quit::Quit { force: false });
    registry.register(quit::Quit { force: true });
    registry.register(open::Open { force: false });
    registry.register(open::Open { force: true });
}
