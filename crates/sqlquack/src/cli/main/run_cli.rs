//! `run_cli`: parse arguments and run a command.

use std::process::ExitCode;

use clap::Parser;

use crate::cli::_helpers::audit_command::{audit_compare_command, audit_sample_command};
use crate::cli::_helpers::census_command::census_command;
use crate::cli::_helpers::pending_command::pending_command;
use crate::cli::_helpers::verify_command::verify_command;
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
        Command::Pending(arguments) => pending_command(arguments, &mut stderr),
        Command::Verify(arguments) => verify_command(arguments, &mut stdout, &mut stderr),
        Command::Audit(arguments) => match &arguments.sample {
            Some(sample) => audit_sample_command(arguments, sample, &mut stderr),
            None => audit_compare_command(arguments, &mut stdout, &mut stderr),
        },
    }
}
