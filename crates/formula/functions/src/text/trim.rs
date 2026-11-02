use excel_model::{CellError, CellValue, Doc};

use super::{text_arg, text_result};
use crate::{Arg, Function};

pub(crate) struct Trim;

impl Function for Trim {
    fn name(&self) -> &'static str {
        "TRIM"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Remove extra spaces",
            usage: "TRIM(text)",
            description: "Removes spaces at the start and end, and leaves single spaces between words.",
            args: &[("text", "Text or a value")],
            examples: &[("=TRIM(\"  a   b \")", "a b")],
            related: &["LEN", "SUBSTITUTE"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [text] = args else {
            return CellValue::Error(CellError::Value);
        };
        text_result(
            text_arg(text).map(|text| text.split(' ').filter(|word| !word.is_empty()).collect::<Vec<_>>().join(" ")),
        )
    }
}
