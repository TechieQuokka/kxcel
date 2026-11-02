use excel_formula_syntax::{Change, adjust, remove_sheet, rename_sheet, translate};

#[test]
fn translation_moves_relative_references() {
    assert_eq!(translate("A1*B1", 1, 0), "A2*B2");
    assert_eq!(translate("SUM(A1:A3)", 0, 2), "SUM(C1:C3)");
}

#[test]
fn translation_keeps_absolute_parts() {
    assert_eq!(translate("$A$1+A$1+$A1", 2, 2), "$A$1+C$1+$A3");
}

#[test]
fn translation_past_the_edge_is_a_ref_error() {
    assert_eq!(translate("A1+1", -1, 0), "#REF!+1");
    assert_eq!(translate("SUM(A1:A3)", -1, 0), "SUM(#REF!)");
}

#[test]
fn translation_leaves_unparsable_text_alone() {
    assert_eq!(translate("1+", 1, 1), "1+");
}

#[test]
fn inserting_rows_moves_references_at_or_below() {
    let change = Change::InsertRows { at: 1, count: 2 };
    assert_eq!(adjust("A1+A2+A3", "S", "S", change), "A1+A4+A5");
    assert_eq!(adjust("$A$2", "S", "S", change), "$A$4");
}

#[test]
fn inserting_inside_a_range_grows_it() {
    let change = Change::InsertRows { at: 2, count: 1 };
    assert_eq!(adjust("SUM(A1:A5)", "S", "S", change), "SUM(A1:A6)");
}

#[test]
fn deleting_a_referenced_cell_is_a_ref_error() {
    let change = Change::DeleteRows { at: 1, count: 1 };
    assert_eq!(adjust("A1+A2+A3", "S", "S", change), "A1+#REF!+A2");
}

#[test]
fn deleting_inside_a_range_shrinks_it() {
    let change = Change::DeleteRows { at: 1, count: 2 };
    assert_eq!(adjust("SUM(A1:A5)", "S", "S", change), "SUM(A1:A3)");
    assert_eq!(adjust("SUM(A2:A5)", "S", "S", change), "SUM(A2:A3)");
    assert_eq!(adjust("SUM(A1:A2)", "S", "S", change), "SUM(A1:A1)");
}

#[test]
fn deleting_a_whole_range_is_a_ref_error() {
    let change = Change::DeleteRows { at: 1, count: 3 };
    assert_eq!(adjust("SUM(A2:A3)", "S", "S", change), "SUM(#REF!)");
}

#[test]
fn columns_work_like_rows() {
    assert_eq!(adjust("A1+C1", "S", "S", Change::InsertCols { at: 1, count: 1 }), "A1+D1");
    assert_eq!(adjust("A1+B1+C1", "S", "S", Change::DeleteCols { at: 1, count: 1 }), "A1+#REF!+B1");
}

#[test]
fn only_references_to_the_changed_sheet_move() {
    let change = Change::InsertRows { at: 0, count: 1 };
    assert_eq!(adjust("A1+Other!A1", "S", "S", change), "A2+Other!A1");
    assert_eq!(adjust("A1+S!A1", "Other", "S", change), "A1+S!A2");
    assert_eq!(adjust("A1+s!A1", "Other", "S", change), "A1+s!A2");
}

#[test]
fn renaming_a_sheet_rewrites_references_to_it() {
    assert_eq!(rename_sheet("매출!A1+SUM(매출!B1:B3)+A1", "매출", "매출2026"), "매출2026!A1+SUM(매출2026!B1:B3)+A1");
    assert_eq!(rename_sheet("data!A1+Other!A1", "Data", "New Name"), "'New Name'!A1+Other!A1");
    assert_eq!(rename_sheet("'Old one'!A1", "old one", "x"), "x!A1");
    assert_eq!(rename_sheet("1+", "a", "b"), "1+");
}

#[test]
fn removing_a_sheet_turns_references_to_it_into_ref_errors() {
    assert_eq!(remove_sheet("매출!A1+A1", "매출"), "#REF!+A1");
    assert_eq!(remove_sheet("SUM(Gone!A1:A3, Kept!A1)", "gone"), "SUM(#REF!,Kept!A1)");
}
