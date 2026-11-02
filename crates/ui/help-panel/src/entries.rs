use excel_model::Doc;

/// One command, function, or key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpEntry {
    pub name: String,
    pub doc: Doc,
}

impl HelpEntry {
    pub fn new(name: impl Into<String>, doc: Doc) -> Self {
        Self { name: name.into(), doc }
    }
}

/// Entries of one category, such as File or Math, listed under a header that
/// folds them away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpGroup {
    pub title: String,
    pub entries: Vec<HelpEntry>,
}

/// A titled kind of entry, such as the commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpSection {
    pub title: String,
    /// Written before names and usage, such as `=` for functions.
    pub prefix: String,
    /// The color of entry names, or the theme's accent if `None`.
    pub color: Option<egui::Color32>,
    pub groups: Vec<HelpGroup>,
}

impl HelpSection {
    /// Groups entries by their category, in the order of `categories`, with
    /// any other categories after them in order of appearance.
    pub fn grouped(title: impl Into<String>, entries: Vec<HelpEntry>, categories: &[&str]) -> Self {
        let mut groups: Vec<HelpGroup> = categories
            .iter()
            .map(|category| HelpGroup { title: (*category).to_owned(), entries: Vec::new() })
            .collect();
        for entry in entries {
            match groups.iter_mut().find(|group| group.title == entry.doc.category) {
                Some(group) => group.entries.push(entry),
                None => groups.push(HelpGroup { title: entry.doc.category.to_owned(), entries: vec![entry] }),
            }
        }
        groups.retain(|group| !group.entries.is_empty());
        Self { title: title.into(), prefix: String::new(), color: None, groups }
    }
}

/// Keeps the entries whose name, summary, or usage contains `query`, ignoring
/// case, and drops groups and sections left empty. A blank query keeps everything.
pub fn filter(sections: &[HelpSection], query: &str) -> Vec<HelpSection> {
    let query = query.trim().to_lowercase();
    let matches = |entry: &HelpEntry| {
        query.is_empty()
            || entry.name.to_lowercase().contains(&query)
            || entry.doc.summary.to_lowercase().contains(&query)
            || entry.doc.usage.to_lowercase().contains(&query)
    };
    sections
        .iter()
        .map(|section| HelpSection {
            groups: section
                .groups
                .iter()
                .map(|group| HelpGroup {
                    title: group.title.clone(),
                    entries: group.entries.iter().filter(|entry| matches(entry)).cloned().collect(),
                })
                .filter(|group| !group.entries.is_empty())
                .collect(),
            ..section.clone()
        })
        .filter(|section| !section.groups.is_empty())
        .collect()
}

/// Finds an entry by its exact name, ignoring case and the section's prefix.
/// Looks in `prefer` first, so a function's related entries stay functions.
pub fn find<'a>(sections: &'a [HelpSection], name: &str, prefer: Option<usize>) -> Option<(usize, &'a HelpEntry)> {
    let name = name.trim();
    let in_section = |index: usize| {
        let section = &sections[index];
        let bare = name.strip_prefix(section.prefix.as_str()).unwrap_or(name);
        section
            .groups
            .iter()
            .flat_map(|group| &group.entries)
            .find(|entry| entry.name.eq_ignore_ascii_case(bare))
            .map(|entry| (index, entry))
    };
    prefer
        .filter(|index| *index < sections.len())
        .and_then(in_section)
        .or_else(|| (0..sections.len()).find_map(in_section))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, category: &'static str, summary: &'static str) -> HelpEntry {
        HelpEntry::new(name, Doc { category, summary, ..Doc::BLANK })
    }

    fn sections() -> Vec<HelpSection> {
        let commands = HelpSection::grouped(
            "Commands",
            vec![
                entry("sort", "Edit", "Sort rows"),
                entry("find", "Navigate", "Move to a match"),
                entry("save", "File", "Save"),
            ],
            &["File", "Edit"],
        );
        let mut functions = HelpSection::grouped(
            "Functions",
            vec![entry("SUM", "Math", "Add numbers"), entry("FIND", "Text", "Position of text")],
            &[],
        );
        functions.prefix = "=".into();
        vec![commands, functions]
    }

    fn names(sections: &[HelpSection]) -> Vec<&str> {
        sections.iter().flat_map(|s| &s.groups).flat_map(|g| g.entries.iter().map(|e| e.name.as_str())).collect()
    }

    #[test]
    fn groups_follow_the_given_order_then_appearance() {
        let sections = sections();
        let titles: Vec<&str> = sections[0].groups.iter().map(|g| g.title.as_str()).collect();
        assert_eq!(titles, ["File", "Edit", "Navigate"]);
    }

    #[test]
    fn a_blank_query_keeps_everything() {
        assert_eq!(filter(&sections(), "  "), sections());
    }

    #[test]
    fn matches_names_and_summaries_ignoring_case() {
        assert_eq!(names(&filter(&sections(), "SORT")), ["sort"]);
        assert_eq!(names(&filter(&sections(), "numbers")), ["SUM"]);
    }

    #[test]
    fn drops_empty_groups_and_sections() {
        let found = filter(&sections(), "sum");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].groups.len(), 1);
        assert!(filter(&sections(), "zzz").is_empty());
    }

    #[test]
    fn finds_exact_names_preferring_a_section() {
        let sections = sections();
        assert_eq!(find(&sections, "Find", None).map(|(i, e)| (i, e.name.as_str())), Some((0, "find")));
        assert_eq!(find(&sections, "FIND", Some(1)).map(|(i, _)| i), Some(1));
        assert_eq!(find(&sections, "=sum", None).map(|(i, _)| i), Some(1));
        assert!(find(&sections, "so", None).is_none());
    }
}
