use std::collections::HashSet;

/// An entry's detail page: its section, and its name there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Page {
    pub section: usize,
    pub name: String,
}

/// Whether help is open, what it is filtered by, and what it shows.
#[derive(Debug, Default)]
pub struct HelpState {
    pub(crate) open: bool,
    pub(crate) query: String,
    pub(crate) search_rect: Option<egui::Rect>,
    /// Groups unfolded by the user, as `section/group` titles.
    pub(crate) expanded: HashSet<String>,
    /// The entry shown in full, if any. The list shows otherwise.
    pub(crate) page: Option<Page>,
    /// The list entry the arrow keys are on, counted among visible entries.
    pub(crate) highlight: Option<usize>,
    /// A topic to open as a page once the entries are known.
    pub(crate) pending_topic: Option<String>,
    /// The tab showing, by index. It stays when help closes and opens again.
    pub(crate) tab: usize,
}

impl HelpState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// The tab showing, by index.
    pub fn tab(&self) -> usize {
        self.tab
    }

    /// The name of the entry shown in full, if any.
    pub fn page_name(&self) -> Option<&str> {
        self.page.as_ref().map(|page| page.name.as_str())
    }

    /// Opens help, filtered by `topic` if there is one. A topic that names an
    /// entry exactly opens its page.
    pub fn open(&mut self, topic: Option<String>) {
        self.open = true;
        self.query = topic.clone().unwrap_or_default();
        self.page = None;
        self.highlight = None;
        self.pending_topic = topic;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open(None);
        }
    }

    /// Where the search box was drawn in the latest frame.
    pub fn search_rect(&self) -> Option<egui::Rect> {
        self.search_rect
    }

    pub(crate) fn toggle_group(&mut self, key: String) {
        if !self.expanded.remove(&key) {
            self.expanded.insert(key);
        }
    }
}
