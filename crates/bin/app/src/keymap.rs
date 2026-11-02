//! Application-wide shortcuts. Shortcuts that act on cells belong to the grid.

use egui::{Context, Key, Modifiers};

/// Whether the help shortcut was pressed.
pub fn help_pressed(ctx: &Context) -> bool {
    ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F1))
}

/// Whether the save shortcut was pressed. Saving runs the `save` command, so it
/// has the same confirmation rules as typing it.
pub fn save_pressed(ctx: &Context) -> bool {
    ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::S))
}
