//! `run_cli`: parse arguments and run a command.

use std::process::ExitCode;

use clap::Parser;

use crate::cli::_helpers::census_command::census_command;
use crate::cli::models::{Cli, Command};

/// Runs `sqlquack` with the process arguments.
pub fn run_cli() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let _ = error.print();
            return if error.use_stderr() {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            };
        }
    };
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    match &cli.command {
        Command::Census(arguments) => census_command(arguments, &mut stdout, &mut stderr),
    }
}
