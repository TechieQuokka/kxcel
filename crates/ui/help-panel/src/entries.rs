/// One line of help.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpEntry {
    pub name: String,
    pub usage: String,
}

impl HelpEntry {
    pub fn new(name: impl Into<String>, usage: impl Into<String>) -> Self {
        Self { name: name.into(), usage: usage.into() }
    }
}

/// A titled group of entries, such as the commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpSection {
    pub title: String,
    pub entries: Vec<HelpEntry>,
    /// The color of entry names, or the theme's accent if `None`.
    pub color: Option<egui::Color32>,
}

/// Keeps the entries whose name or usage contains `query`, ignoring case, and
/// drops sections left empty. A blank query keeps everything.
pub fn filter(sections: &[HelpSection], query: &str) -> Vec<HelpSection> {
    let query = query.trim().to_lowercase();
    sections
        .iter()
        .map(|section| HelpSection {
            title: section.title.clone(),
            color: section.color,
            entries: section
                .entries
                .iter()
                .filter(|entry| {
                    query.is_empty()
                        || entry.name.to_lowercase().contains(&query)
                        || entry.usage.to_lowercase().contains(&query)
                })
                .cloned()
                .collect(),
        })
        .filter(|section| !section.entries.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sections() -> Vec<HelpSection> {
        vec![
            HelpSection {
                title: "Commands".into(),
                entries: vec![HelpEntry::new("sort", "Sort rows"), HelpEntry::new("find", "Move to a match")],
                color: None,
            },
            HelpSection {
                title: "Functions".into(),
                entries: vec![HelpEntry::new("SUM", "SUM(values...)  Add numbers")],
                color: None,
            },
        ]
    }

    fn names(sections: &[HelpSection]) -> Vec<&str> {
        sections.iter().flat_map(|s| s.entries.iter().map(|e| e.name.as_str())).collect()
    }

    #[test]
    fn a_blank_query_keeps_everything() {
        assert_eq!(filter(&sections(), "  "), sections());
    }

    #[test]
    fn matches_names_and_usage_ignoring_case() {
        assert_eq!(names(&filter(&sections(), "SORT")), ["sort"]);
        assert_eq!(names(&filter(&sections(), "numbers")), ["SUM"]);
    }

    #[test]
    fn drops_empty_sections() {
        let found = filter(&sections(), "sum");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Functions");
        assert!(filter(&sections(), "zzz").is_empty());
    }
}
