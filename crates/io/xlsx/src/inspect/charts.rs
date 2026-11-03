/// Finds features stored as their own parts inside the file.
pub(super) fn warnings_from_parts(names: &[String]) -> Vec<String> {
    let has = |prefix: &str| names.iter().any(|name| name.starts_with(prefix));
    let mut warnings = Vec::new();
    for (prefix, label) in [
        ("xl/charts/", "charts"),
        ("xl/media/", "images"),
        ("xl/pivotTables/", "pivot tables"),
        ("xl/comments", "comments"),
        ("xl/vbaProject", "macros"),
    ] {
        if has(prefix) {
            warnings.push(label.to_owned());
        }
    }
    warnings
}
