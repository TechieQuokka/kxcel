use excel_model::CellAddr;

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
}
