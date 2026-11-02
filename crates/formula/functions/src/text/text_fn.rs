use excel_model::{CellError, CellValue, Doc};

use super::format::apply;
use super::{text_arg, text_result, to_text};
use crate::{Arg, Function, scalar};

pub(crate) struct Text;

impl Function for Text {
    fn name(&self) -> &'static str {
        "TEXT"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "A number shown with a format",
            usage: "TEXT(value, format)",
            description: "Shows a number as text in a format. Numbers: 0 and # are digits, a comma groups thousands, a period sets decimal places, and % multiplies by 100, as in #,##0.00 or 0%. Dates: yyyy, yy, m, mm, d, and dd, as in yyyy-mm-dd. Text in quotes is kept as it is.",
            args: &[("value", "A number or date"), ("format", "How to show it, such as \"#,##0\" or \"yyyy-mm-dd\"")],
            examples: &[
                ("=TEXT(1234.5, \"#,##0.00\")", "1,234.50"),
                ("=TEXT(0.256, \"0.0%\")", "25.6%"),
                ("=TEXT(46328, \"yyyy년 m월 d일\")", "2026년 11월 2일"),
            ],
            related: &["VALUE", "ROUND"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [value, format] = args else {
            return CellValue::Error(CellError::Value);
        };
        let result = (|| {
            let format = text_arg(format)?;
            match scalar(value)? {
                CellValue::Number(n) => apply(n, &format),
                CellValue::Text(text) => Ok(text),
                other => to_text(&other),
            }
        })();
        text_result(result)
    }
}
