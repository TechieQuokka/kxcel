use std::path::PathBuf;

pub const USAGE: &str = "\
Usage: kxcel-cli <file.xlsx|file.csv> [-c <command>]... [--print]

Runs commands on a file, in order, stopping at the first failure.
Changes are only written by the save command.

  -c, --command <command>   A command, such as \"sort B\" or \"save\"
  -p, --print               Print the active sheet as tab-separated values
  -h, --help                Show this help";

#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub file: PathBuf,
    pub commands: Vec<String>,
    pub print: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Parsed {
    Run(Args),
    Help,
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Parsed, String> {
    let mut file = None;
    let mut commands = Vec::new();
    let mut print = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Parsed::Help),
            "-c" | "--command" => commands.push(args.next().ok_or("-c needs a command")?),
            "-p" | "--print" => print = true,
            option if option.starts_with('-') => return Err(format!("Unknown option: {option}")),
            _ if file.is_some() => return Err(format!("Unexpected argument: {arg}")),
            _ => file = Some(PathBuf::from(arg)),
        }
    }
    let file = file.ok_or("Missing file name")?;
    if commands.is_empty() && !print {
        return Err("Nothing to do: give -c or --print".to_owned());
    }
    Ok(Parsed::Run(Args { file, commands, print }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(args: &[&str]) -> Result<Parsed, String> {
        parse(args.iter().map(|arg| (*arg).to_owned()))
    }

    #[test]
    fn reads_file_and_commands_in_order() {
        let expected = Args { file: "a.xlsx".into(), commands: vec!["sort B".into(), "save".into()], print: true };
        assert_eq!(parsed(&["a.xlsx", "-c", "sort B", "--command", "save", "-p"]), Ok(Parsed::Run(expected)));
    }

    #[test]
    fn rejects_incomplete_arguments() {
        assert!(parsed(&[]).is_err());
        assert!(parsed(&["a.xlsx"]).is_err());
        assert!(parsed(&["a.xlsx", "-c"]).is_err());
        assert!(parsed(&["a.xlsx", "b.xlsx", "-p"]).is_err());
        assert!(parsed(&["a.xlsx", "--nope"]).is_err());
    }

    #[test]
    fn help_wins() {
        assert_eq!(parsed(&["-h"]), Ok(Parsed::Help));
    }
}
