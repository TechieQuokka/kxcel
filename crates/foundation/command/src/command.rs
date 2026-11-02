use crate::{CommandError, Context};

/// A named action run from the command line.
pub trait Command {
    /// The name typed to run the command.
    fn name(&self) -> &str;
    /// One line shown next to the name during completion.
    fn help(&self) -> &str;
    /// Runs the command with the arguments that followed its name.
    fn run(&self, ctx: &mut Context<'_>, args: &[&str]) -> Result<(), CommandError>;
}
