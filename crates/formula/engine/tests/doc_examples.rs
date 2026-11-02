//! Every example in the help that states a result must give that result.

mod common;

use common::calculated;
use excel_formula_engine::FormulaEngine;

#[test]
fn help_examples_give_the_results_they_state() {
    let mut checked = 0;
    for (name, doc) in FormulaEngine::default().functions() {
        for (example, expected) in doc.examples {
            if expected.is_empty() {
                continue;
            }
            let book = calculated(&[("Z99", example)]);
            let shown = book.active().display("Z99".parse().unwrap());
            assert_eq!(shown, *expected, "{name}: {example}");
            checked += 1;
        }
    }
    assert!(checked > 40, "only {checked} examples have results");
}
