use std::fmt;

use crate::StoreError;

/// Why a command could not run. The message is shown to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandError {
    pub message: String,
    /// Set when the command only stopped to avoid losing something, so a host
    /// can ask the user instead of showing the message.
    pub confirmation: Option<Confirmation>,
}

/// What a command needs confirmed before it goes ahead.
///
/// Each case carries `proceed`, the command line that goes ahead anyway, so a
/// host can run it once the user agrees. Scripts type it themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confirmation {
    /// Unsaved changes would be lost. `action` completes the question "Save
    /// changes before ...?", such as "closing".
    UnsavedChanges { action: String, proceed: String },
    /// Saving over the file would lose this content.
    Loss { items: Vec<String>, proceed: String },
    /// Something that cannot be taken back without undo, such as deleting a
    /// sheet. `question` is asked, and `yes` names the button that goes ahead.
    Ask { question: String, yes: String, proceed: String },
}

impl Confirmation {
    pub fn proceed(&self) -> &str {
        match self {
            Self::UnsavedChanges { proceed, .. } | Self::Loss { proceed, .. } | Self::Ask { proceed, .. } => proceed,
        }
    }
}

impl CommandError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), confirmation: None }
    }

    /// An error that a host may resolve by asking the user.
    pub fn confirm(message: impl Into<String>, confirmation: Confirmation) -> Self {
        Self { message: message.into(), confirmation: Some(confirmation) }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CommandError {}

impl From<StoreError> for CommandError {
    fn from(error: StoreError) -> Self {
        Self::new(error.0)
    }
}
