/// Finds features recorded inside sheet data.
pub(super) fn warnings_from_sheets(sheets: &[String]) -> Vec<String> {
    let any = |needle: &str| sheets.iter().any(|sheet| sheet.contains(needle));
    let mut warnings = Vec::new();
    if sheets.iter().any(|sheet| has_styled_cell(sheet)) {
        warnings.push("cell formatting".to_owned());
    }
    for (needle, label) in [
        ("<mergeCell ", "merged cells"),
        ("<conditionalFormatting", "conditional formatting"),
        ("<dataValidation ", "data validation"),
        ("<hyperlink ", "hyperlinks"),
    ] {
        if any(needle) {
            warnings.push(label.to_owned());
        }
    }
    warnings
}

/// Whether any cell uses a style other than the default.
fn has_styled_cell(sheet: &str) -> bool {
    sheet.split("<c ").skip(1).any(|cell| {
        let attributes = cell.split('>').next().unwrap_or_default();
        attributes
            .split_whitespace()
            .filter_map(|attribute| attribute.strip_prefix("s=\""))
            .any(|style| style.trim_end_matches(['"', '/']) != "0")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_styled_cells() {
        assert!(has_styled_cell(r#"<c r="A1" s="3"><v>1</v></c>"#));
        assert!(has_styled_cell(r#"<c r="A1" s="3"/>"#));
    }

    #[test]
    fn ignores_default_styles_and_values_that_look_like_styles() {
        assert!(!has_styled_cell(r#"<c r="A1" s="0"><v>1</v></c>"#));
        assert!(!has_styled_cell(r#"<c r="A1"><v>1</v></c><col s="3">"#));
    }
}
