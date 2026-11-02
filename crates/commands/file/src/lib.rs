//! Save, exit, and open commands.
//!
//! A command that would lose something stops with a [`Confirmation`], which
//! the GUI turns into a dialog. A trailing `!` goes ahead without asking:
//! `save!` saves over a file with content the application cannot keep, and
//! `exit!` and `open!` discard unsaved changes. Scripts use the `!` forms.
//!
//! [`Confirmation`]: excel_command::Confirmation

mod exit;
mod open;
mod save;

use excel_command::CommandRegistry;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(save::Save { force: false });
    registry.register(save::Save { force: true });
    registry.register(exit::Exit { force: false });
    registry.register(exit::Exit { force: true });
    registry.register(open::Open { force: false });
    registry.register(open::Open { force: true });
}
