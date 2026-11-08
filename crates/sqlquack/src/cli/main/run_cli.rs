//! `run_cli`: parse arguments and run a command.

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;
use sqlquack_core::diagnostics::models::Palette;

use crate::cli::_helpers::audit_command::{
    audit_compare_command, audit_errors_command, audit_sample_command,
};
use crate::cli::_helpers::census_command::census_command;
use crate::cli::_helpers::convert_command::{
    check_command, convert_command, explain_command, fixture_run_command,
};
use crate::cli::_helpers::function_command::function_command;
use crate::cli::_helpers::index_command::index_command;
use crate::cli::_helpers::pending_command::pending_command;
use crate::cli::_helpers::review_command::review_command;
use crate::cli::_helpers::verify_command::{compare_command, verify_command};
use crate::cli::models::{Cli, Command};
use crate::terminal::main::colored_stream::colored_stream;

/// Runs `sqlquack` with the process arguments.
pub fn run_cli() -> ExitCode {
    let cli = match parsed_arguments() {
        Ok(cli) => cli,
        Err(exit) => return exit,
    };
    let choice = match &cli.command {
        Command::Check(arguments) => arguments.colors.color,
        Command::Convert(arguments) => arguments.colors.color,
        _ => clap::ColorChoice::Never,
    };
    let (mut stdout, out_palette) = colored_stream(std::io::stdout().lock(), choice);
    let (mut stderr, palette) = colored_stream(std::io::stderr().lock(), choice);
    run_command(
        &cli.command,
        (&mut stdout, &mut stderr),
        (out_palette, palette),
    )
}

/// Runs `command`, writing to `stdout` and `stderr` in their palettes.
fn run_command(
    command: &Command,
    (stdout, stderr): (&mut dyn Write, &mut dyn Write),
    (out_palette, palette): (&Palette, &Palette),
) -> ExitCode {
    match command {
        Command::Census(arguments) => census_command(arguments, stdout, stderr),
        Command::Pending(arguments) => pending_command(arguments, stderr),
        Command::Verify(arguments) => verify_command(arguments, stdout, stderr),
        Command::Index(arguments) => index_command(arguments, stdout, stderr),
        Command::Review(arguments) => review_command(arguments, stderr),
        Command::Function(arguments) => function_command(arguments, stdout, stderr),
        Command::Convert(arguments) => {
            convert_command(arguments, (stdout, stderr), (out_palette, palette))
        }
        Command::Run(arguments) => fixture_run_command(arguments, (stdout, stderr), palette),
        Command::Check(arguments) => check_command(arguments, stdout, stderr, palette),
        Command::Compare(arguments) => compare_command(arguments, stdout, stderr),
        Command::Explain(arguments) => explain_command(arguments, stdout, stderr),
        Command::Audit(arguments) => match (&arguments.sample, arguments.errors) {
            (Some(sample), _) => audit_sample_command(arguments, sample, stderr),
            (None, true) => audit_errors_command(arguments, stderr),
            (None, false) => audit_compare_command(arguments, stdout, stderr),
        },
    }
}

/// The process arguments, or how to exit once clap has printed its help or error.
fn parsed_arguments() -> Result<Cli, ExitCode> {
    Cli::try_parse().map_err(|error| {
        let _ = error.print();
        match error.use_stderr() {
            true => ExitCode::from(2),
            false => ExitCode::SUCCESS,
        }
    })
}
