use std::collections::HashSet;

/// Finds features recorded inside sheet data. Cells whose style is in `kept`
/// lose nothing on saving.
pub(super) fn warnings_from_sheets(sheets: &[String], kept: &HashSet<String>) -> Vec<String> {
    let any = |needle: &str| sheets.iter().any(|sheet| sheet.contains(needle));
    let mut warnings = Vec::new();
    if sheets.iter().any(|sheet| has_styled_cell(sheet, kept)) {
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

/// Whether any cell uses a style other than the default or a kept one.
fn has_styled_cell(sheet: &str, kept: &HashSet<String>) -> bool {
    sheet.split("<c ").skip(1).any(|cell| {
        let attributes = cell.split('>').next().unwrap_or_default();
        attributes
            .split_whitespace()
            .filter_map(|attribute| attribute.strip_prefix("s=\""))
            .map(|style| style.trim_end_matches(['"', '/']))
            .any(|style| style != "0" && !kept.contains(style))
    })
}

/// The cell styles, by index, that only show a date: a date number format,
/// with the default font, fill, and border, and no alignment. The application
/// keeps dates, so saving loses nothing from these.
pub(super) fn kept_styles(styles: &str) -> HashSet<String> {
    let custom_dates: HashSet<&str> = elements(styles, "numFmt")
        .filter(|element| attribute(element, "formatCode").is_some_and(is_date_code))
        .filter_map(|element| attribute(element, "numFmtId"))
        .collect();
    let Some(cell_xfs) = section(styles, "cellXfs") else {
        return HashSet::new();
    };
    elements(cell_xfs, "xf")
        .enumerate()
        .filter(|(_, element)| {
            let plain = |name| attribute(element, name).is_none_or(|id| id == "0");
            let format = attribute(element, "numFmtId").unwrap_or("0");
            let date = matches!(format, "14" | "15" | "16" | "17") || custom_dates.contains(format);
            date && plain("fontId") && plain("fillId") && plain("borderId") && element.ends_with('/')
        })
        .map(|(index, _)| index.to_string())
        .collect()
}

/// Whether an Excel number format shows a date and nothing more: it has a
/// year or day and no hours or seconds, once quoted text, escaped
/// characters, and bracketed parts are left out.
fn is_date_code(code: &str) -> bool {
    let code = code.replace("&quot;", "\"");
    let mut plain = String::new();
    let (mut quoted, mut bracket, mut escaped) = (false, false, false);
    for c in code.chars() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '"' => quoted = !quoted,
            '[' if !quoted => bracket = true,
            ']' if !quoted => bracket = false,
            c if !quoted && !bracket => plain.push(c.to_ascii_lowercase()),
            _ => {}
        }
    }
    (plain.contains('y') || plain.contains('d')) && !plain.contains('h') && !plain.contains('s')
}

/// The text inside `<name ...>` and `</name>`.
fn section<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let start = xml.find(&format!("<{name}"))?;
    let end = xml[start..].find(&format!("</{name}>"))?;
    Some(&xml[start..start + end])
}

/// The attribute text of each `<name ...>` element, up to its closing `>`. A
/// self-closing element's text ends with `/`.
fn elements<'a>(xml: &'a str, name: &str) -> impl Iterator<Item = &'a str> {
    let open = format!("<{name} ");
    let parts: Vec<&'a str> = xml.split(open.as_str()).skip(1).collect();
    parts.into_iter().map(|rest| rest.split('>').next().unwrap_or_default())
}

fn attribute<'a>(element: &'a str, name: &str) -> Option<&'a str> {
    let start = element.find(&format!("{name}=\""))? + name.len() + 2;
    let end = element[start..].find('"')?;
    Some(&element[start..start + end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_styled_cells() {
        let none = HashSet::new();
        assert!(has_styled_cell(r#"<c r="A1" s="3"><v>1</v></c>"#, &none));
        assert!(has_styled_cell(r#"<c r="A1" s="3"/>"#, &none));
        assert!(!has_styled_cell(r#"<c r="A1" s="3"/>"#, &HashSet::from(["3".to_owned()])));
    }

    #[test]
    fn ignores_default_styles_and_values_that_look_like_styles() {
        let none = HashSet::new();
        assert!(!has_styled_cell(r#"<c r="A1" s="0"><v>1</v></c>"#, &none));
        assert!(!has_styled_cell(r#"<c r="A1"><v>1</v></c><col s="3">"#, &none));
    }

    #[test]
    fn keeps_styles_that_only_show_dates() {
        let styles = r#"<numFmts count="3"><numFmt numFmtId="164" formatCode="yyyy\-mm\-dd"/>
            <numFmt numFmtId="165" formatCode="yyyy-mm-dd hh:mm"/><numFmt numFmtId="166" formatCode="&quot;day&quot;0"/></numFmts>
            <cellXfs count="7"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>
            <xf numFmtId="164" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/>
            <xf numFmtId="14" fontId="0" fillId="0" borderId="0"/>
            <xf numFmtId="164" fontId="1" fillId="0" borderId="0"/>
            <xf numFmtId="165" fontId="0" fillId="0" borderId="0"/>
            <xf numFmtId="14" fontId="0" fillId="0" borderId="0"><alignment horizontal="center"/></xf>
            <xf numFmtId="166" fontId="0" fillId="0" borderId="0"/></cellXfs>"#;
        let mut kept: Vec<String> = kept_styles(styles).into_iter().collect();
        kept.sort();
        assert_eq!(kept, ["1", "2"], "bold, a time, alignment, and text formats are lost");
    }

    #[test]
    fn recognizes_date_codes() {
        for code in ["yyyy-mm-dd", "yyyy\\-mm\\-dd", "d/m/yy", "[$-412]yyyy\"년\" m\"월\" d\"일\""] {
            assert!(is_date_code(code), "{code}");
        }
        for code in ["0.00", "hh:mm", "yyyy-mm-dd hh:mm:ss", "\"days\" 0", "mm:ss"] {
            assert!(!is_date_code(code), "{code}");
        }
    }
}
