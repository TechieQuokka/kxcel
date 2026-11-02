mod common;

use common::calculated;
use excel_model::NumberFormat;

fn shown(cells: &[(&str, &str)], addr: &str) -> (String, NumberFormat) {
    let book = calculated(cells);
    let cell = book.active().get(addr.parse().unwrap()).unwrap().clone();
    (cell.display(), cell.format)
}

#[test]
fn formulas_on_dates_show_dates() {
    let cells = [("A1", "2026-11-02"), ("A2", "2026-12-25"), ("B1", "=A1"), ("B2", "=A1+7"), ("B3", "=30+A1")];
    assert_eq!(shown(&cells, "B1"), ("2026-11-02".into(), NumberFormat::Date));
    assert_eq!(shown(&cells, "B2"), ("2026-11-09".into(), NumberFormat::Date));
    assert_eq!(shown(&cells, "B3"), ("2026-12-02".into(), NumberFormat::Date));
}

#[test]
fn days_between_dates_are_a_number() {
    let cells = [("A1", "2026-11-02"), ("A2", "2026-12-25"), ("B1", "=A2-A1"), ("B2", "=A1*2"), ("B3", "=SUM(A1:A2)")];
    assert_eq!(shown(&cells, "B1"), ("53".into(), NumberFormat::General));
    assert_eq!(shown(&cells, "B2").1, NumberFormat::General);
    assert_eq!(shown(&cells, "B3").1, NumberFormat::General);
}

#[test]
fn a_chain_of_formulas_carries_the_date() {
    let cells = [("A1", "2026-01-31"), ("B1", "=A1+1"), ("C1", "=B1-1")];
    assert_eq!(shown(&cells, "C1"), ("2026-01-31".into(), NumberFormat::Date));
}
