use excel_model::{CellError, CellValue, Doc};

use crate::{Arg, Cells, Function, to_number};

pub(crate) struct SumProduct;

impl Function for SumProduct {
    fn name(&self) -> &'static str {
        "SUMPRODUCT"
    }

    fn doc(&self) -> Doc {
        Doc {
            category: "Math",
            summary: "Multiply ranges position by position, then add",
            usage: "SUMPRODUCT(range1, [range2], ...)",
            description: "Multiplies the cells at the same position in each range and adds the products, such as prices times quantities. All ranges must be the same size. Cells that are not numbers count as 0.",
            args: &[
                ("range1", "Cells to multiply"),
                ("range2", "Optional. More cells of the same size, lined up with range1"),
            ],
            examples: &[("=SUMPRODUCT(B2:B9, C2:C9)", ""), ("=SUMPRODUCT(2, 3)", "6")],
            related: &["SUM", "PRODUCT", "SUMIFS"],
        }
    }

    fn call(&self, args: &[Arg]) -> CellValue {
        match sum_product(args) {
            Ok(total) => super::finite(total),
            Err(error) => CellValue::Error(error),
        }
    }
}

/// One argument: the cells of a range, or a typed number standing for one cell.
enum Array<'a> {
    Cells(&'a Cells),
    Number(f64),
}

impl Array<'_> {
    fn shape(&self) -> (u32, u32) {
        match self {
            Array::Cells(cells) => (cells.rows, cells.cols),
            Array::Number(_) => (1, 1),
        }
    }

    fn at(&self, row: u32, col: u32) -> f64 {
        match self {
            Array::Cells(cells) => match cells.get(row, col) {
                CellValue::Number(n) => *n,
                _ => 0.0,
            },
            Array::Number(n) => *n,
        }
    }
}

fn sum_product(args: &[Arg]) -> Result<f64, CellError> {
    let arrays = args
        .iter()
        .map(|arg| match arg {
            Arg::Cells(cells) => {
                match cells.values().find_map(|value| if let CellValue::Error(e) = value { Some(*e) } else { None }) {
                    Some(error) => Err(error),
                    None => Ok(Array::Cells(cells)),
                }
            }
            Arg::Value(value) => to_number(value).map(Array::Number),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let Some(first) = arrays.first() else {
        return Err(CellError::Value);
    };
    let shape = first.shape();
    if arrays.iter().any(|array| array.shape() != shape) {
        return Err(CellError::Value);
    }
    // An empty cell in the first range makes its product 0, so only its values matter.
    let positions: Vec<(u32, u32)> = match first {
        Array::Cells(cells) => cells.entries().iter().map(|(row, col, _)| (*row, *col)).collect(),
        Array::Number(_) => vec![(0, 0)],
    };
    Ok(positions.into_iter().map(|(row, col)| arrays.iter().map(|array| array.at(row, col)).product::<f64>()).sum())
}
