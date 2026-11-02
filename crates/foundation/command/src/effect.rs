use excel_model::CellAddr;

/// The application's color themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub fn other(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    /// Reads a theme's name, ignoring case.
    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Dark, Self::Light].into_iter().find(|theme| theme.name().eq_ignore_ascii_case(name.trim()))
    }
}

/// A request from a command to its host, the GUI or the CLI.
///
/// Commands do not know about the GUI, so anything beyond the document itself
/// goes through here. A host ignores effects that mean nothing to it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Effect {
    /// Move the cursor to a cell.
    MoveCursor(CellAddr),
    /// Close the application.
    Quit,
    /// A different document was opened, so reset the view.
    DocumentReplaced,
    /// Show the help, filtered to entries matching the text if there is one.
    ShowHelp(Option<String>),
    /// Turn full screen on or off, or switch it when `None`.
    Fullscreen(Option<bool>),
    /// Use a theme, or switch to the other when `None`.
    Theme(Option<Theme>),
}
