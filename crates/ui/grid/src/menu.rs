//! The right-click menu.

use egui::{Area, Frame, Id, Key, Order, Pos2, RichText, Ui};

use crate::{GridAction, GridState};

/// An open right-click menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ContextMenu {
    pub at: Pos2,
    /// True in the frame the menu opened, whose click is not a click outside.
    pub just_opened: bool,
}

/// Shows the menu where it was opened. Choosing an item, `Esc`, or a click
/// elsewhere closes it.
pub(crate) fn show(ui: &Ui, state: &mut GridState, actions: &mut Vec<GridAction>) {
    let Some(menu) = state.menu.as_mut() else {
        return;
    };
    let just_opened = std::mem::take(&mut menu.just_opened);
    let at = menu.at;
    let range = !state.selection().is_single();
    let mut chosen = None;
    let area = Area::new(Id::unique("excel_grid_menu")).order(Order::Foreground).fixed_pos(at).show(ui.ctx(), |ui| {
        Frame::menu(ui.style()).show(ui, |ui| {
            ui.set_min_width(200.0);
            let item = egui::Button::new("Create table").shortcut_text("Ctrl+T");
            if ui.add_enabled(range, item).clicked() {
                chosen = Some("table create");
            }
            if !range {
                ui.label(RichText::new("Select a range first").small().weak());
            }
        })
    });
    let escape = ui.input(|i| i.key_pressed(Key::Escape));
    let clicked_outside = !just_opened
        && ui.input(|i| i.pointer.any_pressed())
        && ui.input(|i| i.pointer.interact_pos()).is_some_and(|pos| !area.response.rect.contains(pos));
    if let Some(line) = chosen {
        actions.push(GridAction::RunCommand(line.into()));
    }
    if chosen.is_some() || escape || clicked_outside {
        state.menu = None;
    }
}
