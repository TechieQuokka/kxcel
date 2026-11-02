/// Whether help is open, and what it is filtered by.
#[derive(Debug, Default)]
pub struct HelpState {
    pub(crate) open: bool,
    pub(crate) query: String,
    pub(crate) search_rect: Option<egui::Rect>,
}

impl HelpState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// Opens help, filtered by `topic` if there is one.
    pub fn open(&mut self, topic: Option<String>) {
        self.open = true;
        self.query = topic.unwrap_or_default();
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
}
