//! `run_cli`: the whole command.

use std::process::ExitCode;

use clap::Parser;

use crate::cli::_helpers::commands::{
    boundaries, diff, family, missing, pages, parse_worker, snapshot_command, stale, summary,
    translation, validate,
};
use crate::cli::_helpers::results_command::results;
use crate::cli::models::{Arguments, Command};

/// Runs one subcommand; progress and problems go to stderr, results to stdout.
pub fn run_cli() -> ExitCode {
    let result: Result<ExitCode, ExitCode> = match Arguments::parse().command {
        Command::Validate { common } => validate(&common),
        Command::Summary { focus, common } => summary(focus.as_deref(), &common),
        Command::Family {
            family: name,
            common,
        } => family(&name, &common),
        Command::Missing { family, common } => missing(family.as_deref(), &common),
        Command::Stale { family, common } => stale(family.as_deref(), &common),
        Command::Pages { family, common } => pages(family.as_deref(), &common),
        Command::Boundaries { form, common } => boundaries(&form, &common),
        Command::Snapshot {
            catalogue_commit,
            common,
        } => snapshot_command(&catalogue_commit, &common),
        Command::Translation { arguments, common } => translation(&arguments, &common),
        Command::Results {
            statements,
            write_baseline,
            report,
            against,
            common,
        } => results(
            &statements,
            (write_baseline, report.as_deref(), against.as_deref()),
            &common,
        ),
        Command::Diff { base, head, common } => diff(&base, head.as_deref(), &common),
        Command::ParseWorker { postgres } => parse_worker(postgres),
    };
    result.unwrap_or_else(|code| code)
}
