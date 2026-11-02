//! Function completion while typing a formula.

use egui::{Area, Frame, Id, Order, Pos2, RichText, Ui};

use crate::GridState;

/// A function offered during formula entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionInfo {
    pub name: String,
    pub help: String,
}

/// The function name being typed at the end of a formula, if any. A formula
/// holding only `=` gives an empty name, which offers every function.
pub fn function_prefix(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('=')?;
    if rest.is_empty() {
        return Some("");
    }
    // Inside a quoted string, letters are just text.
    if rest.matches('"').count() % 2 == 1 {
        return None;
    }
    let is_name_part = |c: char| c.is_alphanumeric() || matches!(c, '_' | '.' | '$' | '!' | '\'');
    let start = rest
        .rfind(|c| !is_name_part(c))
        .map_or(0, |index| index + rest[index..].chars().next().map_or(1, char::len_utf8));
    let token = &rest[start..];
    (!token.is_empty() && token.chars().all(|c| c.is_ascii_alphabetic())).then_some(token)
}

/// Functions whose names start with `prefix`, ignoring case.
pub(crate) fn matching(functions: &[FunctionInfo], prefix: &str) -> Vec<FunctionInfo> {
    let prefix = prefix.to_ascii_uppercase();
    functions.iter().filter(|function| function.name.starts_with(&prefix)).cloned().collect()
}

impl GridState {
    /// Updates the offered functions after the editor has run.
    pub(crate) fn refresh_hints(&mut self, functions: &[FunctionInfo]) {
        if self.text != self.hints_for {
            self.hints_for.clone_from(&self.text);
            self.hints_dismissed = false;
            self.hint_picked = 0;
        }
        self.hints = match function_prefix(&self.text) {
            Some(prefix) if self.editing && !self.hints_dismissed => matching(functions, prefix),
            _ => Vec::new(),
        };
        self.hint_picked = self.hint_picked.min(self.hints.len().saturating_sub(1));
    }

    pub(crate) fn step_hint(&mut self, step: isize) {
        let len = self.hints.len() as isize;
        if len > 0 {
            self.hint_picked = (self.hint_picked as isize + step).rem_euclid(len) as usize;
        }
    }

    /// Replaces the name being typed with the picked function and an opening parenthesis.
    pub(crate) fn accept_hint(&mut self) {
        let Some(function) = self.hints.get(self.hint_picked) else {
            return;
        };
        let name = function.name.clone();
        let typed = function_prefix(&self.text).map_or(0, str::len);
        self.text.truncate(self.text.len() - typed);
        self.text.push_str(&name);
        self.text.push('(');
        self.cursor_to_end = true;
        self.hints.clear();
    }
}

/// Shows the offered functions below the cell. Returns a clicked function's index.
pub(crate) fn show(ui: &Ui, state: &GridState, anchor: Pos2) -> Option<usize> {
    if state.hints.is_empty() {
        return None;
    }
    let mut clicked = None;
    Area::new(Id::unique("excel_function_hints")).order(Order::Foreground).fixed_pos(anchor).show(ui.ctx(), |ui| {
        Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_min_width(320.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new("Functions").small().weak());
            for (index, function) in state.hints.iter().enumerate() {
                let text = if function.help.is_empty() { &function.name } else { &function.help };
                if ui.selectable_label(index == state.hint_picked, RichText::new(text.as_str()).monospace()).clicked() {
                    clicked = Some(index);
                }
            }
        });
    });
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_name_being_typed() {
        for (text, prefix) in [
            ("=", ""),
            ("=su", "su"),
            ("=A1+su", "su"),
            ("=IF(A1>0,ma", "ma"),
            ("=SUM(A1:A2)*mi", "mi"),
            ("=한글+su", "su"),
        ] {
            assert_eq!(function_prefix(text), Some(prefix), "{text}");
        }
    }

    #[test]
    fn offers_nothing_outside_a_function_name() {
        for text in ["su", "", "=A1", "=SUM(", "=1+", "=$A", "=Sheet!A", "=\"su", "=SUM(A1) "] {
            assert_eq!(function_prefix(text), None, "{text}");
        }
    }

    #[test]
    fn matching_ignores_case() {
        let functions: Vec<_> =
            ["SUM", "MAX", "MIN"].map(|name| FunctionInfo { name: name.into(), help: String::new() }).to_vec();
        assert_eq!(matching(&functions, "mi").len(), 1);
        assert_eq!(matching(&functions, "m").len(), 2);
        assert_eq!(matching(&functions, "").len(), 3);
    }
}
