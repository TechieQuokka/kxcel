use excel_model::{CellError, CellValue, Doc};

use super::{text_arg, text_result};
use crate::{Arg, Function};

pub(crate) struct Lower;

impl Function for Lower {
    fn name(&self) -> &'static str {
        "LOWER"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Text in small letters",
            usage: "LOWER(text)",
            description: "Changes letters to small letters.",
            args: &[("text", "Text or a value")],
            examples: &[("=LOWER(\"APPLE\")", "apple")],
            related: &["UPPER"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [text] = args else {
            return CellValue::Error(CellError::Value);
        };
        text_result(text_arg(text).map(|text| text.to_lowercase()))
    }
}
