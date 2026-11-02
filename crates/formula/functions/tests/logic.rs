use excel_formula_functions::{Arg, FunctionRegistry};
use excel_model::{CellError, CellValue};

fn call_if(args: &[Arg]) -> CellValue {
    FunctionRegistry::with_builtins().get("IF").unwrap().call(args)
}

fn value(v: CellValue) -> Arg {
    Arg::Value(v)
}

fn text(s: &str) -> CellValue {
    CellValue::Text(s.into())
}

#[test]
fn picks_a_branch() {
    let args = |condition| [value(CellValue::Bool(condition)), value(text("yes")), value(text("no"))];
    assert_eq!(call_if(&args(true)), text("yes"));
    assert_eq!(call_if(&args(false)), text("no"));
}

#[test]
fn numbers_work_as_conditions() {
    assert_eq!(call_if(&[value(CellValue::Number(2.0)), value(text("yes"))]), text("yes"));
}

#[test]
fn a_missing_else_gives_false() {
    assert_eq!(call_if(&[value(CellValue::Bool(false)), value(text("yes"))]), CellValue::Bool(false));
}

#[test]
fn ignores_errors_in_the_branch_not_taken() {
    let error = CellValue::Error(CellError::Div0);
    let args = [value(CellValue::Bool(true)), value(text("ok")), value(error.clone())];
    assert_eq!(call_if(&args), text("ok"));
    let args = [value(CellValue::Bool(false)), value(text("ok")), value(error.clone())];
    assert_eq!(call_if(&args), error);
}

#[test]
fn an_error_condition_propagates() {
    let args = [value(CellValue::Error(CellError::Na)), value(text("a")), value(text("b"))];
    assert_eq!(call_if(&args), CellValue::Error(CellError::Na));
}

#[test]
fn text_conditions_are_value_errors() {
    let args = [value(text("maybe")), value(text("a"))];
    assert_eq!(call_if(&args), CellValue::Error(CellError::Value));
}

#[test]
fn wrong_argument_counts_are_value_errors() {
    assert_eq!(call_if(&[value(CellValue::Bool(true))]), CellValue::Error(CellError::Value));
    let four = [value(CellValue::Bool(true)), value(text("a")), value(text("b")), value(text("c"))];
    assert_eq!(call_if(&four), CellValue::Error(CellError::Value));
}

#[test]
fn single_cell_arguments_are_their_value() {
    let args = [Arg::cells(vec![CellValue::Bool(true)]), Arg::cells(vec![text("a")])];
    assert_eq!(call_if(&args), text("a"));
}

#[test]
fn an_empty_cell_branch_shows_zero() {
    let args = [value(CellValue::Bool(true)), Arg::cells(Vec::new())];
    assert_eq!(call_if(&args), CellValue::Number(0.0));
}
