//! Finds things in a file that saving would lose.

mod charts;
mod styles;

use std::fs::File;
use std::io::Read;
use std::path::Path;

use excel_command::StoreError;
use zip::ZipArchive;

use crate::error::file_error;

/// Lists what the application would lose by saving over this file. An empty
/// list means saving is safe.
pub fn inspect(path: &Path) -> Result<Vec<String>, StoreError> {
    let file = File::open(path).map_err(|e| file_error("open", path, e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| file_error("read", path, e))?;
    let names: Vec<String> = archive
        .file_names()
        .map(|name| name.map(|name| name.into_owned()))
        .collect::<Result<_, _>>()
        .map_err(|e| file_error("read", path, e))?;

    let mut warnings = charts::warnings_from_parts(&names);
    let mut sheets = Vec::new();
    for name in names.iter().filter(|name| is_sheet_part(name)) {
        sheets.push(read_part(&mut archive, name).map_err(|e| file_error("read", path, e))?);
    }
    let kept = match names.iter().any(|name| name == "xl/styles.xml") {
        true => {
            styles::kept_styles(&read_part(&mut archive, "xl/styles.xml").map_err(|e| file_error("read", path, e))?)
        }
        false => Default::default(),
    };
    warnings.extend(styles::warnings_from_sheets(&sheets, &kept));
    if names.iter().any(|name| name == "xl/workbook.xml") {
        let meta = crate::tables::read_meta(&mut archive).map_err(|e| file_error("read", path, e))?;
        warnings.extend(meta.warnings);
        let workbook = read_part(&mut archive, "xl/workbook.xml").map_err(|e| file_error("read", path, e))?;
        if workbook.contains("<definedName") {
            warnings.push("defined names".to_owned());
        }
    }
    Ok(warnings)
}

fn is_sheet_part(name: &str) -> bool {
    name.starts_with("xl/worksheets/") && name.ends_with(".xml")
}

fn read_part(archive: &mut ZipArchive<File>, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut text = String::new();
    archive.by_name(name)?.read_to_string(&mut text)?;
    Ok(text)
}
