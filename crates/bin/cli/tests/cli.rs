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
fn version_prints_the_version() {
    for flag in ["--version", "-V"] {
        let output = cli(&[flag]);
        assert!(output.status.success());
        assert_eq!(stdout(&output).trim(), format!("kxcel-cli {}", env!("CARGO_PKG_VERSION")));
    }
    assert!(stdout(&cli(&["--help"])).contains("--version"));
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

#[test]
fn window_commands_say_they_need_the_app() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("t.xlsx");
    let output = cli(&[file.to_str().unwrap(), "-c", "fullscreen on", "-c", "theme light", "-c", "set A1 1", "-p"]);
    assert!(output.status.success(), "the remaining commands still run");
    assert!(
        stderr(&output).contains("full screen only") && stderr(&output).contains("themes only"),
        "{}",
        stderr(&output)
    );
    assert_eq!(stdout(&output), "1\n");
}

#[test]
fn renaming_and_deleting_sheets_rewrites_formulas() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("t.xlsx");
    let output = cli(&[
        file.to_str().unwrap(),
        "-c",
        "sheet add 매출",
        "-c",
        "set A1 5",
        "-c",
        "sheet add Temp",
        "-c",
        "set A1 7",
        "-c",
        "sheet 1",
        "-c",
        "set A1 =매출!A1*2",
        "-c",
        "set B1 =Temp!A1",
        "-c",
        "sheet 2",
        "-c",
        "sheet rename 매출 2026",
        "-c",
        "sheet 3",
        "-c",
        "sheet delete",
    ]);
    assert_eq!(output.status.code(), Some(1), "delete asks first");
    assert!(stderr(&output).contains("sheet delete!"), "{}", stderr(&output));

    let mut commands: Vec<&str> = Vec::new();
    for command in [
        "sheet add 매출",
        "set A1 5",
        "sheet add Temp",
        "set A1 7",
        "sheet 1",
        "set A1 =매출!A1*2",
        "set B1 =Temp!A1",
        "sheet 2",
        "sheet rename 매출 2026",
        "sheet 3",
        "sheet delete!",
        "sheet 1",
    ] {
        commands.extend(["-c", command]);
    }
    let mut args = vec![file.to_str().unwrap()];
    args.extend(commands);
    args.push("-p");
    let output = cli(&args);
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    assert!(printed.contains("Renamed sheet 매출 to 매출 2026\nSheet Temp\nDeleted sheet Temp"), "{printed}");
    assert!(
        printed.ends_with("\n10\t#REF!\n"),
        "the renamed sheet still answers, and the deleted one is #REF!: {printed}"
    );
}

#[test]
fn an_app_opens_on_the_sheet_saved_showing_and_install_says_when_it_has_no_input_cells() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("form.xlsx");
    let install = |commands: &[&str]| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kxcel-cli"));
        command.env("XDG_DATA_HOME", dir.path().join("data")).arg(&file);
        for line in commands {
            command.args(["-c", line]);
        }
        let output = command.output().unwrap();
        assert!(output.status.success(), "{}", stderr(&output));
        stdout(&output)
    };

    let said = install(&["sheet add 견적", "set A1 수량", "app input B1", "save", "app install 견적 계산기"]);
    assert!(said.contains("Registered app 견적 계산기"), "{said}");
    assert!(!said.contains("no input cells"), "{said}");
    let reopened = cli(&[file.to_str().unwrap(), "-c", "app input"]);
    assert!(stdout(&reopened).contains("Input cells: B1"), "{}", stdout(&reopened));

    let said = install(&["sheet 1", "save", "app install 견적 계산기"]);
    assert!(said.contains("It opens on sheet Sheet1, which has no input cells"), "{said}");
}
