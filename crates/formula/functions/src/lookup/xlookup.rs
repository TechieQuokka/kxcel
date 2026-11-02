use std::cmp::Ordering;

use excel_model::{CellError, CellValue, Doc};

use super::{compare, equals, line, shown};
use crate::math::cells;
use crate::{Arg, Function, scalar, scalar_number};

pub(crate) struct XLookup;

impl Function for XLookup {
    fn name(&self) -> &'static str {
        "XLOOKUP"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Lookup",
            summary: "Find a value and give the matching one",
            usage: "XLOOKUP(value, look_in, give_from, [if_not_found], [match], [order])",
            description: "Looks for value in look_in, a single row or column, and gives the value at the same position in give_from, which must line up with it. Unlike VLOOKUP, it finds an exact match by default and the data need not be sorted. Match 0 is exact, -1 takes the next smaller value when there is no exact one, 1 the next larger, and 2 allows * and ? in text. Order -1 searches from the end.",
            args: &[
                ("value", "What to look for"),
                ("look_in", "A single row or column to search"),
                ("give_from", "Cells lined up with look_in to give a value from"),
                ("if_not_found", "Optional. The result when nothing is found, instead of #N/A"),
                ("match", "Optional. 0 exact, -1 next smaller, 1 next larger, 2 wildcards. 0 by default"),
                ("order", "Optional. 1 from the start, -1 from the end. 1 by default"),
            ],
            examples: &[("=XLOOKUP(\"사과\", A2:A20, C2:C20)", ""), ("=XLOOKUP(E1, A1:A100, B1:B100, \"없음\")", "")],
            related: &["VLOOKUP", "INDEX", "MATCH"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        let [wanted, look_in, give_from, rest @ ..] = args else {
            return CellValue::Error(CellError::Value);
        };
        let result = (|| {
            let wanted = scalar(wanted)?;
            let (look_in, give_from) = (cells(look_in)?, cells(give_from)?);
            let across = match (look_in.rows, look_in.cols) {
                (_, 1) => false,
                (1, _) => true,
                _ => return Err(CellError::Value),
            };
            let lined_up = if across {
                give_from.rows == 1 && give_from.cols == look_in.cols
            } else {
                give_from.cols == 1 && give_from.rows == look_in.rows
            };
            if !lined_up || rest.len() > 3 {
                return Err(CellError::Value);
            }
            let number = |index: usize, default: f64| rest.get(index).map_or(Ok(default), scalar_number);
            let (mode, order) = (number(1, 0.0)?, number(2, 1.0)?);
            let mut values = line(look_in, 0, across);
            if order < 0.0 {
                values.reverse();
            }
            match find(&wanted, &values, mode) {
                Some(at) => Ok(shown(if across { give_from.get(0, at) } else { give_from.get(at, 0) })),
                None => match rest.first() {
                    Some(fallback) => scalar(fallback),
                    None => Err(CellError::Na),
                },
            }
        })();
        result.unwrap_or_else(CellValue::Error)
    }
}

/// The position of the first exact match, or for `mode` -1 or 1 the closest
/// smaller or larger value when there is none. The data need not be sorted.
fn find(wanted: &CellValue, values: &[(u32, &CellValue)], mode: f64) -> Option<u32> {
    let wildcards = mode == 2.0;
    if let Some((at, _)) = values.iter().find(|(_, value)| equals(wanted, value, wildcards)) {
        return Some(*at);
    }
    let side = if mode < 0.0 {
        Ordering::Less
    } else if mode > 0.0 && !wildcards {
        Ordering::Greater
    } else {
        return None;
    };
    let mut best: Option<(u32, &CellValue)> = None;
    for (at, value) in values {
        if compare(value, wanted) != Some(side) {
            continue;
        }
        // Closer to the value wanted is better, and the first found wins ties.
        if best.is_none_or(|(_, current)| compare(value, current) == Some(side.reverse())) {
            best = Some((*at, value));
        }
    }
    best.map(|(at, _)| at)
}
