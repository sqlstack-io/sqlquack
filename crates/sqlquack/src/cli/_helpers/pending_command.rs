//! `sqlquack pending`.

use std::io::Write;
use std::process::ExitCode;

use sqlquack_evidence::pending::main::write_pending::write_pending;
use sqlquack_evidence::replay::main::duckdb_version::duckdb_version;

use crate::cli::_helpers::evidence::load_evidence;
use crate::cli::models::PendingArguments;

pub(crate) fn pending_command(arguments: &PendingArguments, stderr: &mut dyn Write) -> ExitCode {
    let Some(evidence) = load_evidence("pending", &arguments.paths, stderr) else {
        return ExitCode::FAILURE;
    };
    let written = duckdb_version().and_then(|version| {
        write_pending(
            &evidence.cases,
            &arguments.paths.results,
            &arguments.out,
            &version,
        )
    });
    match written {
        Ok(summary) => {
            let _ = writeln!(
                stderr,
                "sqlquack pending: done, {} pending case files in {} ({} cases already recorded, {} stale files removed)",
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
