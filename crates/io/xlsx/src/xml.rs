//! Just enough XML reading and writing for the parts of a file that the
//! libraries leave out. The parts are machine-written, so plain text search
//! is enough.

/// The text inside `<name ...>` and `</name>`.
pub(crate) fn section<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let start = find_element(xml, name)?;
    let end = xml[start..].find(&format!("</{name}>"))?;
    Some(&xml[start..start + end])
}

/// Where the element `<name` starts, not counting longer names that begin
/// the same way, such as `<filters` for `<filter`.
fn find_element(xml: &str, name: &str) -> Option<usize> {
    let open = format!("<{name}");
    let mut from = 0;
    while let Some(found) = xml[from..].find(&open) {
        let at = from + found;
        let next = xml[at + open.len()..].chars().next();
        if matches!(next, Some(' ' | '>' | '/' | '\n' | '\r' | '\t')) {
            return Some(at);
        }
        from = at + open.len();
    }
    None
}

/// The attribute text of each `<name ...>` element, up to its closing `>`. A
/// self-closing element's text ends with `/`.
pub(crate) fn elements<'a>(xml: &'a str, name: &str) -> impl Iterator<Item = &'a str> {
    let open = format!("<{name} ");
    let parts: Vec<&'a str> = xml.split(open.as_str()).skip(1).collect();
    parts.into_iter().map(|rest| rest.split('>').next().unwrap_or_default())
}

/// Each `<name ...>...</name>` element whole, attributes and contents. A
/// self-closing one has no contents.
pub(crate) fn whole_elements<'a>(xml: &'a str, name: &str) -> Vec<&'a str> {
    let mut found = Vec::new();
    let mut rest = xml;
    while let Some(start) = find_element(rest, name) {
        let tail = &rest[start..];
        let head_end = tail.find('>').map_or(tail.len(), |end| end + 1);
        let end = if tail[..head_end].ends_with("/>") {
            head_end
        } else {
            let close = format!("</{name}>");
            tail.find(&close).map_or(tail.len(), |end| end + close.len())
        };
        found.push(&tail[..end]);
        rest = &tail[end..];
    }
    found
}

/// An attribute's value, with entities decoded.
pub(crate) fn attribute(element: &str, name: &str) -> Option<String> {
    raw_attribute(element, name).map(unescape)
}

/// An attribute's value as written in the file.
pub(crate) fn raw_attribute<'a>(element: &'a str, name: &str) -> Option<&'a str> {
    let mut from = 0;
    let pattern = format!("{name}=\"");
    while let Some(found) = element[from..].find(&pattern) {
        let at = from + found;
        // Only a whole attribute name, not the end of a longer one.
        let whole = at == 0 || element[..at].ends_with(|c: char| c.is_whitespace());
        let start = at + pattern.len();
        if whole {
            let end = element[start..].find('"')?;
            return Some(&element[start..start + end]);
        }
        from = start;
    }
    None
}

/// Writes text for an attribute value.
pub(crate) fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            c => escaped.push(c),
        }
    }
    escaped
}

/// Decodes the entities XML attribute values may hold.
pub(crate) fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let Some(end) = tail.find(';') else {
            out.push_str(tail);
            return out;
        };
        let entity = &tail[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => out.push(c),
            None => out.push_str(&tail[..=end]),
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_whole_names_only() {
        let xml = r#"<filters blank="1"><filter val="a&amp;b"/></filters><filter val="c"/>"#;
        let filters = whole_elements(xml, "filter");
        assert_eq!(filters, [r#"<filter val="a&amp;b"/>"#, r#"<filter val="c"/>"#]);
        assert_eq!(attribute(filters[0], "val").as_deref(), Some("a&b"));
        assert_eq!(whole_elements(xml, "filters").len(), 1);
        assert_eq!(raw_attribute(r#"<t displayName="T" name="N"/>"#, "name"), Some("N"));
    }

    #[test]
    fn escapes_round_trip() {
        let text = r#"<서울 & "부산">"#;
        assert_eq!(unescape(&escape(text)), text);
        assert_eq!(unescape("&#44032;&#xAC00;&bogus;"), "가가&bogus;");
    }
}
