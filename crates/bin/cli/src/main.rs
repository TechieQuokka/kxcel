//! Headless command runner, using the same commands as the GUI's command line.

mod args;
mod runner;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args = match args::parse(std::env::args().skip(1)) {
        Ok(args::Parsed::Run(args)) => args,
        Ok(args::Parsed::Help) => {
            println!("{}\n\n{}", args::USAGE, runner::help_text(None).trim_end());
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("{error}\n\n{}", args::USAGE);
            return ExitCode::from(2);
        }
    };
    match runner::run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
