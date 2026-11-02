use excel_command::CommandError;

/// Reads an optional `on` or `off`: `None` means switch to the other state.
pub(crate) fn on_off(args: &[&str], usage: &str) -> Result<Option<bool>, CommandError> {
    match args {
        [] => Ok(None),
        [word] if word.eq_ignore_ascii_case("on") => Ok(Some(true)),
        [word] if word.eq_ignore_ascii_case("off") => Ok(Some(false)),
        _ => Err(CommandError::new(format!("Usage: {usage}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_on_off_or_nothing() {
        assert_eq!(on_off(&[], "x"), Ok(None));
        assert_eq!(on_off(&["ON"], "x"), Ok(Some(true)));
        assert_eq!(on_off(&["off"], "x"), Ok(Some(false)));
        assert!(on_off(&["maybe"], "x").is_err());
        assert!(on_off(&["on", "off"], "x").is_err());
    }
}
