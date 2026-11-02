use std::fmt::Display;
use std::path::Path;

use excel_command::StoreError;

/// Builds an error naming the file and what went wrong.
pub(crate) fn file_error(action: &str, path: &Path, cause: impl Display) -> StoreError {
    StoreError(format!("Cannot {action} {}: {cause}", path.display()))
}

/// Rejects anything but `.xlsx`, which is the only format this crate handles.
pub(crate) fn require_xlsx(path: &Path) -> Result<(), StoreError> {
    let is_xlsx = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("xlsx"));
    if is_xlsx { Ok(()) } else { Err(StoreError(format!("Only .xlsx files are supported: {}", path.display()))) }
}
