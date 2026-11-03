/// Help for a command or function, shown in the help panel and by `help <name>`.
///
/// Only `summary` and `usage` are required. Build one from [`Doc::BLANK`]:
///
/// ```
/// use excel_model::Doc;
/// const DOC: Doc = Doc { category: "Math", summary: "Add numbers", usage: "SUM(values...)", ..Doc::BLANK };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Doc {
    /// The group it is listed under, such as `File` or `Math`.
    pub category: &'static str,
    /// One line, shown in lists and completion.
    pub summary: &'static str,
    /// How to write it, such as `sort <column> [desc]` or `SUM(values...)`.
    pub usage: &'static str,
    /// A few sentences of detail.
    pub description: &'static str,
    /// Each argument and what it means. Optional ones say so.
    pub args: &'static [(&'static str, &'static str)],
    /// Examples, each with its result, or an empty result when it has none.
    pub examples: &'static [(&'static str, &'static str)],
    /// Names of related entries.
    pub related: &'static [&'static str],
}

impl Doc {
    pub const BLANK: Doc =
        Doc { category: "Other", summary: "", usage: "", description: "", args: &[], examples: &[], related: &[] };

    /// The whole entry as plain text, for the command line tool.
    pub fn to_text(&self) -> String {
        let mut text = format!("{}\n  {}\n", self.usage, self.summary);
        if !self.description.is_empty() {
            text.push('\n');
            // Every line is indented, and blank lines between paragraphs stay blank.
            for line in self.description.lines() {
                if line.is_empty() {
                    text.push('\n');
                } else {
                    text.push_str(&format!("  {line}\n"));
                }
            }
        }
        if !self.args.is_empty() {
            text.push_str("\nArguments:\n");
            let width = self.args.iter().map(|(name, _)| name.chars().count()).max().unwrap_or(0);
            for (name, meaning) in self.args {
                text.push_str(&format!("  {name:<width$}  {meaning}\n"));
            }
        }
        if !self.examples.is_empty() {
            text.push_str("\nExamples:\n");
            for (example, result) in self.examples {
                match *result {
                    "" => text.push_str(&format!("  {example}\n")),
                    result => text.push_str(&format!("  {example}  ->  {result}\n")),
                }
            }
        }
        if !self.related.is_empty() {
            text.push_str(&format!("\nSee also: {}\n", self.related.join(", ")));
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_indents_every_line_of_the_description() {
        let doc = Doc { usage: "x", summary: "s", description: "First.\n\nSteps:\n1. One.", ..Doc::BLANK };
        assert_eq!(doc.to_text(), "x\n  s\n\n  First.\n\n  Steps:\n  1. One.\n");
    }

    #[test]
    fn plain_text_shows_every_part_given() {
        let doc = Doc {
            summary: "Add numbers",
            usage: "SUM(values...)",
            args: &[("values", "Numbers or ranges")],
            examples: &[("=SUM(1, 2)", "3"), ("=SUM(A1:A3)", "")],
            related: &["AVERAGE"],
            ..Doc::BLANK
        };
        assert_eq!(
            doc.to_text(),
            "SUM(values...)\n  Add numbers\n\nArguments:\n  values  Numbers or ranges\n\n\
             Examples:\n  =SUM(1, 2)  ->  3\n  =SUM(A1:A3)\n\nSee also: AVERAGE\n"
        );
    }

    #[test]
    fn plain_text_leaves_out_missing_parts() {
        let doc = Doc { summary: "Exit", usage: "exit", ..Doc::BLANK };
        assert_eq!(doc.to_text(), "exit\n  Exit\n");
    }
}
