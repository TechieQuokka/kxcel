use egui::text::LayoutJob;
use egui::{Color32, FontId, Frame, Id, Key, Margin, Modifiers, RichText, ScrollArea, TextEdit, TextFormat, Ui};

use crate::state::Page;
use crate::{HelpEntry, HelpSection, HelpState, filter, find};

/// Something the user did in the help panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpAction {
    /// The user clicked into the search box.
    FocusSearch,
    /// The user pressed Escape in the search box on the list.
    Leave,
    /// The user closed the panel.
    Close,
}

const DEFAULT_WIDTH: f32 = 340.0;

/// Draws the panel on the right if help is open. `search_focused` says whether
/// the search box should have keyboard focus.
///
/// The panel shows a list of entries grouped by category, or one entry's page.
/// Groups start folded and unfold while searching. With the search box focused,
/// the arrow keys move through the list, Enter opens a page, and Escape goes
/// back from a page, then leaves.
pub fn show(ui: &mut Ui, state: &mut HelpState, sections: &[HelpSection], search_focused: bool) -> Option<HelpAction> {
    if !state.open {
        state.search_rect = None;
        return None;
    }
    if let Some(topic) = state.pending_topic.take()
        && let Some((section, entry)) = find(sections, &topic, None)
    {
        state.page = Some(Page { section, name: entry.name.clone() });
    }

    let id = Id::unique("excel_help_search");
    let found = filter(sections, &state.query);
    let visible = visible_entries(state, sections, &found);
    let mut action = None;

    if search_focused {
        // Requesting focus interrupts input method composition, so only ask when needed.
        if !ui.memory(|memory| memory.has_focus(id)) {
            ui.memory_mut(|memory| memory.request_focus(id));
        }
        action = keys_before_search(ui, state, &visible);
    }

    let fill = ui.visuals().panel_fill;
    egui::Panel::right("excel_help_panel")
        .default_size(DEFAULT_WIDTH)
        .resizable(true)
        .frame(Frame::NONE.fill(fill).inner_margin(Margin::same(10)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Help").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("x").on_hover_text("Close (F1)").clicked() {
                        action = Some(HelpAction::Close);
                    }
                });
            });
            ui.add_space(6.0);

            let before = state.query.clone();
            let search = TextEdit::singleline(&mut state.query)
                .id(id)
                .hint_text("Search")
                .desired_width(f32::INFINITY)
                .return_key(None)
                .show(ui)
                .response;
            state.search_rect = Some(search.rect);
            if !search_focused && search.clicked() {
                action = Some(HelpAction::FocusSearch);
            }
            if state.query != before {
                // Typing goes back to the list, to show what matches.
                state.page = None;
                state.highlight = None;
            }
            ui.add_space(8.0);

            // Read after the text box, so text an input method commits with Enter counts.
            if search_focused && state.page.is_none() && ui.input(|i| i.key_pressed(Key::Enter)) {
                let chosen = state.highlight.or((visible.len() == 1).then_some(0));
                if let Some((section, name)) = chosen.and_then(|index| visible.get(index)) {
                    state.page = Some(Page { section: *section, name: name.clone() });
                }
            }

            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let page = state.page.clone();
                let entry = page.as_ref().and_then(|page| find(sections, &page.name, Some(page.section)));
                match entry {
                    Some((section, entry)) => show_page(ui, state, sections, section, entry),
                    None => show_list(ui, state, sections, &found),
                }
            });
        });

    action
}

/// The entries the list shows, in order, as section index and name.
fn visible_entries(state: &HelpState, sections: &[HelpSection], found: &[HelpSection]) -> Vec<(usize, String)> {
    let searching = !state.query.trim().is_empty();
    let mut visible = Vec::new();
    for section in found {
        let index = section_index(sections, section);
        for group in &section.groups {
            if searching || state.expanded.contains(&group_key(section, &group.title)) {
                visible.extend(group.entries.iter().map(|entry| (index, entry.name.clone())));
            }
        }
    }
    visible
}

/// Where a filtered section sits among all sections, found by its title.
fn section_index(sections: &[HelpSection], section: &HelpSection) -> usize {
    sections.iter().position(|other| other.title == section.title).unwrap_or(0)
}

fn group_key(section: &HelpSection, group: &str) -> String {
    format!("{}/{group}", section.title)
}

/// Handles keys the search box would otherwise act on.
fn keys_before_search(ui: &Ui, state: &mut HelpState, visible: &[(usize, String)]) -> Option<HelpAction> {
    ui.input_mut(|i| {
        if i.consume_key(Modifiers::NONE, Key::Escape) {
            if state.page.take().is_some() {
                return None;
            }
            return Some(HelpAction::Leave);
        }
        if state.page.is_none() && !visible.is_empty() {
            let last = visible.len() - 1;
            if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                state.highlight = Some(state.highlight.map_or(0, |index| (index + 1).min(last)));
            } else if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                state.highlight = Some(state.highlight.map_or(0, |index| index.saturating_sub(1)));
            }
        }
        None
    })
}

fn show_list(ui: &mut Ui, state: &mut HelpState, sections: &[HelpSection], found: &[HelpSection]) {
    if found.is_empty() {
        ui.label(RichText::new("No matches").weak());
        return;
    }
    let searching = !state.query.trim().is_empty();
    let mut position = 0;
    for section in found {
        ui.label(RichText::new(&section.title).strong());
        ui.add_space(2.0);
        let color = section.color.unwrap_or(ui.visuals().selection.stroke.color);
        for group in &section.groups {
            let key = group_key(section, &group.title);
            let unfolded = searching || state.expanded.contains(&key);
            let arrow = if unfolded { "\u{25BE}" } else { "\u{25B8}" };
            let header = format!("{arrow} {} ({})", group.title, group.entries.len());
            if ui.selectable_label(false, RichText::new(header)).clicked() && !searching {
                state.toggle_group(key);
            }
            if !unfolded {
                continue;
            }
            for entry in &group.entries {
                let highlighted = state.highlight == Some(position);
                let row = ui.selectable_label(highlighted, entry_line(ui, section, entry, color));
                if highlighted {
                    row.scroll_to_me(None);
                }
                if row.clicked() {
                    state.page = Some(Page { section: section_index(sections, section), name: entry.name.clone() });
                }
                position += 1;
            }
        }
        ui.add_space(10.0);
    }
}

/// An entry's list line: its name in the section's color, then its summary.
fn entry_line(ui: &Ui, section: &HelpSection, entry: &HelpEntry, color: Color32) -> LayoutJob {
    let mono = FontId::monospace(13.0);
    let body = egui::TextStyle::Body.resolve(ui.style());
    let mut job = LayoutJob::default();
    job.append(&format!("    {}{}", section.prefix, entry.name), 0.0, TextFormat::simple(mono, color));
    job.append(&format!("  {}", entry.doc.summary), 0.0, TextFormat::simple(body, ui.visuals().weak_text_color()));
    job
}

fn show_page(ui: &mut Ui, state: &mut HelpState, sections: &[HelpSection], section_index: usize, entry: &HelpEntry) {
    let section = &sections[section_index];
    let doc = &entry.doc;
    let color = section.color.unwrap_or(ui.visuals().selection.stroke.color);
    let prefix = section.prefix.as_str();

    if ui.button("\u{2190} Back").on_hover_text("Back to the list (Esc)").clicked() {
        state.page = None;
        return;
    }
    ui.add_space(8.0);
    ui.label(RichText::new(format!("{prefix}{}", entry.name)).monospace().size(18.0).color(color));
    ui.label(RichText::new(format!("{} \u{00B7} {}", section.title, doc.category)).small().weak());
    ui.add_space(4.0);
    ui.label(doc.summary);

    heading(ui, "Usage");
    ui.label(RichText::new(format!("{prefix}{}", doc.usage)).monospace());

    if !doc.description.is_empty() {
        ui.add_space(8.0);
        ui.label(doc.description);
    }
    if !doc.args.is_empty() {
        heading(ui, "Arguments");
        egui::Grid::new("excel_help_args").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            for (name, meaning) in doc.args {
                ui.label(RichText::new(*name).monospace());
                ui.label(*meaning);
                ui.end_row();
            }
        });
    }
    if !doc.examples.is_empty() {
        heading(ui, "Examples");
        for (example, result) in doc.examples {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(*example).monospace());
                if !result.is_empty() {
                    ui.label(RichText::new(format!("\u{2192} {result}")).weak());
                }
            });
        }
    }
    if !doc.related.is_empty() {
        heading(ui, "See also");
        ui.horizontal_wrapped(|ui| {
            for name in doc.related {
                let label = RichText::new(format!("{prefix}{name}")).monospace().color(color);
                if ui.link(label).clicked()
                    && let Some((section, found)) = find(sections, name, Some(section_index))
                {
                    state.page = Some(Page { section, name: found.name.clone() });
                }
            }
        });
    }
}

fn heading(ui: &mut Ui, text: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(text).small().strong());
    ui.add_space(2.0);
}
