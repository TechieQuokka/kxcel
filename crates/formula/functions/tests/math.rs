use excel_formula_functions::{Arg, FunctionRegistry};
use excel_model::{CellError, CellValue};

fn call(name: &str, args: &[Arg]) -> CellValue {
    FunctionRegistry::with_builtins().get(name).unwrap().call(args)
}

fn number(n: f64) -> CellValue {
    CellValue::Number(n)
}

fn cells() -> Arg {
    Arg::Cells(vec![number(1.0), CellValue::Text("x".into()), number(5.0), CellValue::Bool(true)])
}

#[test]
fn sum_ignores_text_and_booleans_in_cells() {
    assert_eq!(call("SUM", &[cells()]), number(6.0));
}

#[test]
fn sum_converts_typed_values() {
    let args = [Arg::Value(CellValue::Text("2".into())), Arg::Value(CellValue::Bool(true))];
    assert_eq!(call("SUM", &args), number(3.0));
}

#[test]
fn sum_rejects_typed_text_that_is_not_a_number() {
    let args = [Arg::Value(CellValue::Text("x".into()))];
    assert_eq!(call("SUM", &args), CellValue::Error(CellError::Value));
}

#[test]
fn errors_in_cells_propagate() {
    let args = [Arg::Cells(vec![number(1.0), CellValue::Error(CellError::Na)])];
    for name in ["SUM", "AVERAGE", "MIN", "MAX"] {
        assert_eq!(call(name, &args), CellValue::Error(CellError::Na), "{name}");
    }
}

#[test]
fn average_divides_by_the_count_of_numbers() {
    assert_eq!(call("AVERAGE", &[cells()]), number(3.0));
}

#[test]
fn average_of_nothing_is_div0() {
    assert_eq!(call("AVERAGE", &[Arg::Cells(Vec::new())]), CellValue::Error(CellError::Div0));
}

#[test]
fn count_counts_numbers_and_skips_errors() {
    let args = [
        cells(),
        Arg::Cells(vec![CellValue::Error(CellError::Na)]),
        Arg::Value(number(9.0)),
        Arg::Value(CellValue::Text("x".into())),
    ];
    assert_eq!(call("COUNT", &args), number(3.0));
}

#[test]
fn min_and_max_find_extremes() {
    assert_eq!(call("MIN", &[cells()]), number(1.0));
    assert_eq!(call("MAX", &[cells()]), number(5.0));
}

#[test]
fn min_and_max_of_nothing_are_zero() {
    assert_eq!(call("MIN", &[Arg::Cells(Vec::new())]), number(0.0));
    assert_eq!(call("MAX", &[Arg::Cells(Vec::new())]), number(0.0));
}
