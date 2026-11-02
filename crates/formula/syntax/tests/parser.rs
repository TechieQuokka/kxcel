use excel_formula_syntax::{BinaryOp, CellRef, Expr, UnaryOp, parse};

/// Parses and displays, which shows how the formula was grouped.
fn normalized(formula: &str) -> String {
    parse(formula).unwrap().to_string()
}

#[test]
fn follows_operator_precedence() {
    let Expr::Binary(BinaryOp::Add, _, right) = parse("1+2*3").unwrap() else {
        panic!("expected addition at the top");
    };
    assert!(matches!(*right, Expr::Binary(BinaryOp::Mul, ..)));
}

#[test]
fn operators_are_left_associative() {
    let Expr::Binary(BinaryOp::Sub, left, _) = parse("1-2-3").unwrap() else {
        panic!("expected subtraction at the top");
    };
    assert!(matches!(*left, Expr::Binary(BinaryOp::Sub, ..)));

    let Expr::Binary(BinaryOp::Pow, left, _) = parse("2^3^2").unwrap() else {
        panic!("expected power at the top");
    };
    assert!(matches!(*left, Expr::Binary(BinaryOp::Pow, ..)), "Excel evaluates ^ left to right");
}

#[test]
fn negation_binds_tighter_than_power() {
    let Expr::Binary(BinaryOp::Pow, left, _) = parse("-2^2").unwrap() else {
        panic!("expected power at the top");
    };
    assert!(matches!(*left, Expr::Unary(UnaryOp::Neg, _)), "Excel evaluates -2^2 as 4");
}

#[test]
fn reads_percent_as_a_postfix() {
    assert!(matches!(parse("50%").unwrap(), Expr::Unary(UnaryOp::Percent, _)));
    assert_eq!(normalized("A1*50%"), "A1*50%");
}

#[test]
fn reads_references() {
    let Expr::Range { sheet, start, end } = parse("Data!$A$1:B2").unwrap() else {
        panic!("expected a range");
    };
    assert_eq!(sheet.as_deref(), Some("Data"));
    assert_eq!(start, CellRef { addr: "A1".parse().unwrap(), abs_row: true, abs_col: true });
    assert_eq!(end, CellRef { addr: "B2".parse().unwrap(), abs_row: false, abs_col: false });
}

#[test]
fn reads_mixed_references() {
    let Expr::Cell { cell, .. } = parse("A$1").unwrap() else {
        panic!("expected a cell");
    };
    assert!(cell.abs_row && !cell.abs_col);
    assert_eq!(cell.to_string(), "A$1");
}

#[test]
fn function_names_become_uppercase() {
    assert_eq!(normalized("sum(a1, 2)"), "SUM(A1,2)");
    assert!(matches!(parse("true").unwrap(), Expr::Bool(true)));
}

#[test]
fn displaying_keeps_meaning() {
    for formula in [
        "(1+2)*3",
        "1-(2-3)",
        "2^(3^2)",
        "-(1+2)",
        "IF(A1>0,\"yes\",\"no\")",
        "'My Sheet'!A1+시트!B2",
        "\"a\"\"b\"&C3",
        "(A1+B1)%",
        "NOW()",
    ] {
        assert_eq!(normalized(formula), formula);
        assert_eq!(parse(&normalized(formula)), parse(formula), "{formula}");
    }
}

#[test]
fn rejects_malformed_formulas() {
    for formula in ["", "1+", "(1", "1)", "SUM(1", "SUM(1,", "foo", "A1:", "A1:foo", "Sheet!", "1 2", "'x'"] {
        assert!(parse(formula).is_err(), "{formula}");
    }
}

#[test]
fn survives_deep_nesting_of_reasonable_size() {
    let formula = format!("{}1{}", "(".repeat(200), ")".repeat(200));
    assert!(parse(&formula).is_ok());
}
