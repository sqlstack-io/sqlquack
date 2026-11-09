//! `sqlquack pending`.

use std::io::Write;
use std::process::ExitCode;

use sqlquack_evidence::pending::main::write_pending::write_pending;
use sqlquack_evidence::pending::models::PendingSummary;
use sqlquack_evidence::replay::main::engine_version::engine_version;

use crate::cli::_helpers::evidence::load_evidence;
use crate::cli::models::PendingArguments;

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
