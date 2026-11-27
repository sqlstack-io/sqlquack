//! `sqlquack pending`, and `sqlquack import-databricks`, which records pending cases from a journal.

use std::io::Write;
use std::process::ExitCode;

use sqlquack_evidence::databricks::journal::main::import_databricks_journal::import_databricks_journal;
use sqlquack_evidence::pending::main::write_pending::write_pending;
use sqlquack_evidence::pending::models::PendingSummary;
use sqlquack_evidence::replay::main::engine_version::engine_version;

use crate::cli::_helpers::evidence::load_evidence;
use crate::cli::models::{ImportDatabricksArguments, PendingArguments};

pub(crate) fn pending_command(arguments: &PendingArguments, stderr: &mut dyn Write) -> ExitCode {
    let _ = writeln!(stderr, "sqlquack pending: loading the policy and cases…");
    match pending(arguments) {
        Ok((cases, summary)) => {
            let _ = writeln!(
                stderr,
                "sqlquack pending: done, {} pending case files in {} ({cases} cases, {} already recorded, {} stale files removed)",
                summary.written,
                arguments.out.display(),
                summary.recorded,
                summary.removed
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack pending: failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn pending(arguments: &PendingArguments) -> Result<(usize, PendingSummary), String> {
    let evidence = load_evidence(&arguments.paths)?;
    let version = engine_version().map_err(|error| error.to_string())?;
    let summary = write_pending(
        &evidence.cases,
        &arguments.paths.results,
        &arguments.out,
        &version,
    )
    .map_err(|error| error.to_string())?;
    Ok((evidence.cases.len(), summary))
}

pub(crate) fn import_databricks_command(
    arguments: &ImportDatabricksArguments,
    stderr: &mut dyn Write,
) -> ExitCode {
    let _ = writeln!(
        stderr,
        "sqlquack import-databricks: reading {} journals and the pending cases…",
        arguments.journal.len()
    );
    let summary = match import_databricks_journal(
        &arguments.journal,
        (&arguments.pending, &arguments.results),
        &arguments.warehouse_version,
    ) {
        Ok(summary) => summary,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack import-databricks: failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    for (reference, reason) in &summary.not_recordable {
        let _ = writeln!(
            stderr,
            "sqlquack import-databricks: left pending {reference}: {reason}"
        );
    }
    let _ = writeln!(
        stderr,
        "sqlquack import-databricks: done, {} results with rows and {} with an error written to {}; \
         {} already recorded, {} left pending, {} statements answer no pending case",
        summary.rows,
        summary.errors,
        arguments.results.display(),
        summary.already_recorded,
        summary.not_recordable.len(),
        summary.unmatched
    );
    ExitCode::SUCCESS
}
