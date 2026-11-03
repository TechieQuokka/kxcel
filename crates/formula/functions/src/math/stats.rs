//! Statistics over numbers: `MEDIAN`, `LARGE`, `SMALL`, `RANK`, and `STDEV`.
//! They collect numbers as `SUM` does, so text and blanks in ranges are skipped.

use excel_model::{CellError, CellValue, Doc};

use super::{cells, numbers};
use crate::{Arg, Function, scalar_number};

/// Numbers sorted from smallest to largest.
fn sorted(args: &[Arg]) -> Result<Vec<f64>, CellError> {
    let mut values = numbers(args)?;
    values.sort_by(f64::total_cmp);
    Ok(values)
}

pub(crate) struct Median;

impl Function for Median {
    fn name(&self) -> &'static str {
        "MEDIAN"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Middle number",
            usage: "MEDIAN(values...)",
            description: "The middle of the numbers given, once sorted, or the average of the two middle ones when there is an even count. Unlike AVERAGE, one very large value does not pull it. With no numbers, the result is #NUM!.",
            args: &[("values", "Numbers, cells, or ranges, separated by commas")],
            examples: &[("=MEDIAN(3, 1, 9)", "3"), ("=MEDIAN(1, 2, 3, 100)", "2.5")],
            related: &["AVERAGE", "LARGE", "SMALL"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match sorted(args) {
            Ok(values) if values.is_empty() => CellValue::Error(CellError::Num),
            Ok(values) => {
                let middle = values.len() / 2;
                let median =
                    if values.len() % 2 == 1 { values[middle] } else { (values[middle - 1] + values[middle]) / 2.0 };
                CellValue::Number(median)
            }
            Err(error) => CellValue::Error(error),
        }
    }
}

/// The `k`th number counting from the largest when `largest`, or from the smallest.
fn kth(args: &[Arg], largest: bool) -> Result<f64, CellError> {
    let [values, k] = args else {
        return Err(CellError::Value);
    };
    let values = sorted(std::slice::from_ref(values))?;
    let k = scalar_number(k)?.trunc();
    if k < 1.0 || k > values.len() as f64 {
        return Err(CellError::Num);
    }
    let k = k as usize;
    Ok(if largest { values[values.len() - k] } else { values[k - 1] })
}

pub(crate) struct Large;

impl Function for Large {
    fn name(&self) -> &'static str {
        "LARGE"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "The kth largest number",
            usage: "LARGE(values, k)",
            description: "The kth largest number in values: 1 is the largest, 2 the second largest, and so on. k is cut to a whole number, and below 1 or past the count of numbers it is #NUM!.",
            args: &[("values", "Cells or a range of numbers"), ("k", "Which one, counting from the largest")],
            examples: &[("=LARGE(B2:B20, 2)", ""), ("=LARGE(B2:B20, 1) + LARGE(B2:B20, 2) + LARGE(B2:B20, 3)", "")],
            related: &["SMALL", "MAX", "RANK"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        kth(args, true).map_or_else(CellValue::Error, CellValue::Number)
    }
}

pub(crate) struct Small;

impl Function for Small {
    fn name(&self) -> &'static str {
        "SMALL"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "The kth smallest number",
            usage: "SMALL(values, k)",
            description: "The kth smallest number in values: 1 is the smallest, 2 the second smallest, and so on. k is cut to a whole number, and below 1 or past the count of numbers it is #NUM!.",
            args: &[("values", "Cells or a range of numbers"), ("k", "Which one, counting from the smallest")],
            examples: &[("=SMALL(B2:B20, 1)", "")],
            related: &["LARGE", "MIN", "RANK"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        kth(args, false).map_or_else(CellValue::Error, CellValue::Number)
    }
}

pub(crate) struct Rank;

impl Function for Rank {
    fn name(&self) -> &'static str {
        "RANK"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Rank of a number in a list",
            usage: "RANK(number, range, [order])",
            description: "Where number stands among the numbers in range: 1 for the largest, or with order 1 for the smallest. Equal numbers share a rank, and the next rank is skipped. A number not in range is #N/A.",
            args: &[
                ("number", "The number to rank"),
                ("range", "Cells with the numbers to rank against"),
                ("order", "Optional. 0 or left out ranks from the largest, anything else from the smallest"),
            ],
            examples: &[("=RANK(B2, B$2:B$20)", ""), ("=RANK(B2, B$2:B$20, 1)", "")],
            related: &["LARGE", "SMALL"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let result = (|| {
            let (number, range, ascending) = match args {
                [number, range] => (number, range, false),
                [number, range, order] => (number, range, scalar_number(order)? != 0.0),
                _ => return Err(CellError::Value),
            };
            let number = scalar_number(number)?;
            cells(range)?;
            let values = numbers(std::slice::from_ref(range))?;
            if !values.contains(&number) {
                return Err(CellError::Na);
            }
            let ahead = values.iter().filter(|v| if ascending { **v < number } else { **v > number }).count();
            Ok((ahead + 1) as f64)
        })();
        result.map_or_else(CellValue::Error, CellValue::Number)
    }
}

pub(crate) struct StDev;

impl Function for StDev {
    fn name(&self) -> &'static str {
        "STDEV"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Standard deviation of a sample",
            usage: "STDEV(values...)",
            description: "How far the numbers spread from their average, treating them as a sample, as Excel's STDEV and STDEV.S do. With fewer than two numbers, the result is #DIV/0!.",
            args: &[("values", "Numbers, cells, or ranges, separated by commas")],
            examples: &[("=STDEV(2, 4, 6)", "2"), ("=STDEV(B2:B20)", "")],
            related: &["AVERAGE", "MEDIAN"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match numbers(args) {
            Ok(values) if values.len() < 2 => CellValue::Error(CellError::Div0),
            Ok(values) => {
                let count = values.len() as f64;
                let mean = values.iter().sum::<f64>() / count;
                let squares: f64 = values.iter().map(|v| (v - mean).powi(2)).sum();
                super::finite((squares / (count - 1.0)).sqrt())
            }
            Err(error) => CellValue::Error(error),
        }
    }
}
