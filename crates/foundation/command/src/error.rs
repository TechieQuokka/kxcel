use std::fmt;

use crate::StoreError;

/// Why a command could not run. The text is shown to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandError(pub String);

impl CommandError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CommandError {}

impl From<StoreError> for CommandError {
    fn from(error: StoreError) -> Self {
        Self(error.0)
    }
}
