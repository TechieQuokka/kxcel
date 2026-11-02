use excel_model::Doc;

use crate::{CommandError, Context};

/// A named action run from the command line.
pub trait Command {
    /// The name typed to run the command.
    fn name(&self) -> &str;
    /// Help for the command. Its summary shows next to the name during completion.
    fn doc(&self) -> Doc;
    /// Runs the command with the arguments that followed its name.
    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError>;
    /// Whether the argument is a file path, so completion offers files.
    fn takes_path(&self) -> bool {
        false
    }
    /// Whether the command gets the rest of its line as one argument, exactly
    /// as typed, instead of split into words. Quotes then stay as they are,
    /// which a formula needs.
    fn takes_raw_line(&self) -> bool {
        false
    }
}
