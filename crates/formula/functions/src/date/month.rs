use excel_model::{CellError, CellValue, Doc};

use super::parts;
use crate::{Arg, Function};

pub(crate) struct Month;

impl Function for Month {
    fn name(&self) -> &'static str {
        "MONTH"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Date",
            summary: "Month of a date",
            usage: "MONTH(date)",
            description: "The month of a date, as a number.",
            args: &[("date", "A date")],
            examples: &[("=MONTH(DATE(2026, 11, 2))", "11")],
            related: &["DATE", "YEAR", "MONTH", "DAY"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [date] = args else {
            return CellValue::Error(CellError::Value);
        };
        parts(date).map_or_else(CellValue::Error, |(_, month, _)| CellValue::Number(month as f64))
    }
}
