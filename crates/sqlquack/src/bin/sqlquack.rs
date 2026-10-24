//! `sqlquack`: the command line.

use std::process::ExitCode;

use sqlquack::cli::main::run_cli::run_cli;

fn main() -> ExitCode {
    run_cli()
}
