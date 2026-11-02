//! Runs the real `kxcel-cli` binary on real files.

use std::path::Path;
use std::process::{Command, Output};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kxcel-cli")).args(args).output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn run_ok(file: &Path, commands: &[&str]) -> Output {
    let mut args = vec![file.to_str().unwrap()];
    for command in commands {
        args.extend(["-c", command]);
    }
    args.push("-p");
    let output = cli(&args);
    assert!(output.status.success(), "{}", stderr(&output));
    output
}

#[test]
fn creates_calculates_saves_and_reopens_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("new.xlsx");
    let output = run_ok(&file, &["set A1 3", "set A2 4", "set A3 =SUM(A1:A2)*2", "set B1 사과", "save"]);
    assert_eq!(stdout(&output).lines().next_back(), Some("14\t"));
    assert!(stdout(&output).contains("3\t사과"));

    let reopened = cli(&[file.to_str().unwrap(), "-p"]);
    assert_eq!(stdout(&reopened), "3\t사과\n4\t\n14\t\n");
}

#[test]
fn sorting_and_structure_keep_formulas_correct() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("t.xlsx");
    run_ok(&file, &["set A1 3", "set B1 =A1*10", "set A2 1", "set B2 =A2*10", "set A3 =SUM(B1:B2)", "save"]);

    let selected = run_ok(&file, &["sort A desc"]);
    assert!(stderr(&selected).contains("not saved"));

    // Row 3 moves to row 4, and the total's range follows the inserted row.
    let output = run_ok(&file, &["insert row 2", "set A2 2", "set B2 =A2*10"]);
    assert_eq!(stdout(&output).lines().last(), Some("60\t"));

    let output = run_ok(&file, &["delete row 1"]);
    assert_eq!(stdout(&output).lines().last(), Some("10\t"));
}

#[test]
fn the_cursor_persists_between_commands() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("t.xlsx");
    let output = run_ok(&file, &["set A1 1", "set A2 2", "goto A2", "insert row"]);
    assert_eq!(stdout(&output), "Inserted 1 row at 2\n1\n\n2\n");
}

#[test]
fn stops_at_the_first_failure_with_a_nonzero_exit() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("t.xlsx");
    let output = cli(&[file.to_str().unwrap(), "-c", "nope", "-c", "save"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("Unknown command: nope"));
    assert!(!file.exists());
}

#[test]
fn bad_usage_exits_with_two() {
    assert_eq!(cli(&[]).status.code(), Some(2));
    assert_eq!(cli(&["a.xlsx"]).status.code(), Some(2));
}

#[test]
fn help_lists_options() {
    let output = cli(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("--command"));
    assert!(stdout(&output).contains("sort"), "lists commands");
    assert!(stdout(&output).contains("=SUM("), "lists functions with their =");
}

#[test]
fn the_help_command_prints_matching_entries() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("t.xlsx");
    let output = cli(&[file.to_str().unwrap(), "-c", "help sum"]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("=SUM(values...)"), "{text}");
    assert!(text.contains("Examples:") && text.contains("See also"), "an exact name prints its page: {text}");
    assert!(!text.contains("sort"), "{text}");

    let listed = stdout(&cli(&[file.to_str().unwrap(), "-c", "help sa"]));
    assert!(listed.contains("  File\n") && listed.contains("save [file]"), "{listed}");
    assert!(!listed.contains("Examples:"), "{listed}");
}

#[test]
fn a_formatted_file_warns_and_needs_save_bang() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("styled.xlsx");
    let mut writer = rust_xlsxwriter::Workbook::new();
    let bold = rust_xlsxwriter::Format::new().set_bold();
    writer.add_worksheet().write_number_with_format(0, 0, 1.0, &bold).unwrap();
    writer.save(&file).unwrap();
    let path = file.to_str().unwrap();

    let opened = cli(&[path, "-p"]);
    assert!(stderr(&opened).contains("cell formatting"));

    let refused = cli(&[path, "-c", "save"]);
    assert_eq!(refused.status.code(), Some(1));
    assert!(stderr(&refused).contains("save!"));

    let forced = cli(&[path, "-c", "save!"]);
    assert!(forced.status.success(), "{}", stderr(&forced));
    assert!(!stderr(&cli(&[path, "-p"])).contains("cell formatting"));
}

#[test]
fn set_keeps_quotes_in_formulas() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("t.xlsx");
    let path = file.to_str().unwrap();
    let output = cli(&[
        path,
        "-c",
        "set A1 사과 주스",
        "-c",
        r#"set B1 =IF(A1="사과 주스", "맞음", "아님")"#,
        "-c",
        r#"set C1 =COUNTIF(A1:A9, "사과*")"#,
        "-p",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "사과 주스\t맞음\t1\n");
}
