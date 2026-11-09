//! `syntax-coverage`: checks documented Snowflake syntax coverage.

use std::process::ExitCode;

use sqlquack_coverage_check::cli::main::run_cli::run_cli;

fn main() -> ExitCode {
    run_cli()
}
