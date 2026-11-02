use excel_model::{CellError, CellValue, Doc, date_serial};

use crate::{Arg, Function};

pub(crate) struct Today;

impl Function for Today {
    fn name(&self) -> &'static str {
        "TODAY"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Date",
            summary: "Today's date",
            usage: "TODAY()",
            description: "Today's date in the computer's time zone. It is computed again after every edit, so it stays current.",
            args: &[],
            examples: &[],
            related: &["DATE"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        if !args.is_empty() {
            return CellValue::Error(CellError::Value);
        }
        let today = jiff::Zoned::now().date();
        match date_serial(i64::from(today.year()), today.month() as u32, today.day() as u32) {
            Some(serial) => CellValue::Number(serial),
            None => CellValue::Error(CellError::Num),
        }
    }

    fn returns_date(&self) -> bool {
        true
    }

    fn volatile(&self) -> bool {
        true
    }
}
