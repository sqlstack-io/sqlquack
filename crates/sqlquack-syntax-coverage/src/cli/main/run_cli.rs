//! `run_cli`: the whole command.

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::models::{Arguments, Command};
use crate::formats::main::load_coverage::load_coverage;

/// Runs one subcommand; progress and problems go to stderr.
pub fn run_cli() -> ExitCode {
    let arguments: Arguments = Arguments::parse();
    let mut stderr = std::io::stderr().lock();
    let Command::Validate { root } = arguments.command;
    let _ = writeln!(stderr, "syntax-coverage: validating {}", root.display());
    match load_coverage(&root) {
        Ok(coverage) => {
            let forms: usize = coverage.catalogue.values().map(Vec::len).sum();
            let _ = writeln!(
                stderr,
                "syntax-coverage: valid, {forms} forms, {} receipts",
                coverage.receipts.len()
            );
            ExitCode::SUCCESS
        }
        Err(problems) => {
            for problem in &problems {
                let _ = writeln!(stderr, "syntax-coverage: {problem}");
            }
            let _ = writeln!(
                stderr,
                "syntax-coverage: FAILED, {} problems",
                problems.len()
            );
            ExitCode::FAILURE
        }
    }
}
