use excel_model::{CellError, CellValue, Doc};

use super::{text_arg, text_result};
use crate::{Arg, Function};

pub(crate) struct Upper;

impl Function for Upper {
    fn name(&self) -> &'static str {
        "UPPER"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Text",
            summary: "Text in capitals",
            usage: "UPPER(text)",
            description: "Changes letters to capitals. Korean has no case, so it stays.",
            args: &[("text", "Text or a value")],
            examples: &[("=UPPER(\"apple\")", "APPLE")],
            related: &["LOWER"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [text] = args else {
            return CellValue::Error(CellError::Value);
        };
        text_result(text_arg(text).map(|text| text.to_uppercase()))
    }
}
