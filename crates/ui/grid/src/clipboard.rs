use excel_model::{CellAddr, CellRange, Sheet};

/// Renders a range as tab-separated values as the grid shows them, commas
/// included, the format other
/// spreadsheets exchange through the clipboard. A value holding a tab, a line
/// break, or a quote is quoted, as Excel does. Hidden rows are left out.
pub fn to_tsv(sheet: &Sheet, range: CellRange) -> String {
    let mut text = String::new();
    let mut first = true;
    for row in (range.start.row..=range.end.row).filter(|row| !sheet.is_row_hidden(*row)) {
        if !first {
            text.push('\n');
        }
        first = false;
        for col in range.start.col..=range.end.col {
            if col > range.start.col {
                text.push('\t');
            }
            text.push_str(&quote_field(&sheet.shown(CellAddr::new(row, col))));
        }
    }
    text
}

/// Quotes a field if it holds a tab, a line break, or a quote.
pub fn quote_field(value: &str) -> String {
    if value.contains(['\t', '\n', '\r', '"']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

/// Splits clipboard text into rows of cells. Quoted fields may hold tabs and
/// line breaks. A trailing line break adds no row.
pub fn parse_tsv(text: &str) -> Vec<Vec<String>> {
    let mut rows = vec![Vec::new()];
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    let mut at_field_start = true;
    while let Some(c) = chars.next() {
        match c {
            '"' if at_field_start => {
                at_field_start = false;
                while let Some(c) = chars.next() {
                    if c != '"' {
                        field.push(c);
                    } else if chars.next_if_eq(&'"').is_some() {
                        field.push('"');
                    } else {
                        break;
                    }
                }
            }
            '\t' => {
                rows.last_mut().expect("there is always a row").push(std::mem::take(&mut field));
                at_field_start = true;
            }
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' => {
                rows.last_mut().expect("there is always a row").push(std::mem::take(&mut field));
                rows.push(Vec::new());
                at_field_start = true;
            }
            c => {
                field.push(c);
                at_field_start = false;
            }
        }
    }
    let trailing_break = at_field_start && field.is_empty() && rows.len() > 1 && rows.last().is_some_and(Vec::is_empty);
    if trailing_break {
        rows.pop();
    } else {
        rows.last_mut().expect("there is always a row").push(field);
    }
    rows
}

#[cfg(test)]
mod tests {
    use excel_model::{Cell, CellValue};

    use super::*;

    #[test]
    fn renders_blank_cells_as_empty_fields() {
        let mut sheet = Sheet::new("S");
        sheet.set("A1".parse().unwrap(), Some(Cell::literal(CellValue::Number(1.0))));
        sheet.set("B2".parse().unwrap(), Some(Cell::literal(CellValue::Text("사과".into()))));
        assert_eq!(to_tsv(&sheet, "A1:B2".parse().unwrap()), "1\t\n\t사과");
    }

    #[test]
    fn parses_rows_and_cells() {
        assert_eq!(parse_tsv("1\t2\r\n3\t\n"), vec![vec!["1", "2"], vec!["3", ""]]);
        assert_eq!(parse_tsv("x"), vec![vec!["x"]]);
        assert_eq!(parse_tsv(""), vec![vec![""]]);
    }

    #[test]
    fn multi_line_values_survive_a_round_trip() {
        let mut sheet = Sheet::new("S");
        sheet.set("A1".parse().unwrap(), Cell::parse_input("first\nsecond"));
        sheet.set("B1".parse().unwrap(), Cell::parse_input("say \"hi\""));
        sheet.set("A2".parse().unwrap(), Cell::parse_input("plain"));
        let text = to_tsv(&sheet, "A1:B2".parse().unwrap());
        assert_eq!(text, "\"first\nsecond\"\t\"say \"\"hi\"\"\"\nplain\t");
        assert_eq!(parse_tsv(&text), vec![vec!["first\nsecond", "say \"hi\""], vec!["plain", ""]]);
    }
}
