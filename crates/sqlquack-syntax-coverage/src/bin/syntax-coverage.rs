//! `syntax-coverage`: checks documented Snowflake syntax coverage.

use std::process::ExitCode;

use sqlquack_syntax_coverage::cli::main::run_cli::run_cli;

fn main() -> ExitCode {
    run_cli()
}
