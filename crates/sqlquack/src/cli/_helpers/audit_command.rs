//! `sqlquack audit`: `--sample` writes pending files for re-recording; `--compare` checks them.

use std::io::Write;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlquack_evidence::audit::main::compare_audit::compare_audit;
use sqlquack_evidence::audit::main::sample_cases::sample_cases;
use sqlquack_evidence::audit::models::{AuditEntry, AuditOutcome};
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_evidence::pending::main::write_pending::write_pending;
use sqlquack_evidence::replay::main::duckdb_version::duckdb_version;
use sqlquack_evidence::results::main::load_result::load_result;
use sqlquack_evidence::results::models::RecordedOutcome;

use crate::cli::_helpers::evidence::{load_evidence, parse_percent};
use crate::cli::models::AuditArguments;

pub(crate) fn audit_sample_command(
    arguments: &AuditArguments,
    sample: &str,
    stderr: &mut dyn Write,
) -> ExitCode {
    let percent = match parse_percent(sample) {
        Ok(percent) => percent,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack audit: {error}");
            return ExitCode::from(2);
        }
    };
    let seed = arguments.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs())
    });
    let _ = writeln!(stderr, "sqlquack audit: sampling recorded cases…");
    match sample_pending(arguments, percent, seed) {
        Ok((sampled, written)) => {
            let _ = writeln!(
                stderr,
                "sqlquack audit: done, sampled {sampled} recorded cases with seed {seed}; {written} pending case files in {}",
                arguments.out.display()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack audit: failed: {error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn audit_compare_command(
    arguments: &AuditArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let _ = writeln!(
        stderr,
        "sqlquack audit: comparing re-recorded results with stored ones…"
    );
    let entries = match compare(arguments) {
        Ok(entries) => entries,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack audit: failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let rendered = if arguments.json {
        serde_json::to_string_pretty(&entries).unwrap_or_default() + "\n"
    } else {
        render(&entries)
    };
    let _ = stdout.write_all(rendered.as_bytes());
    let problems = entries
        .iter()
        .filter(|entry| entry.outcome != AuditOutcome::Matched)
        .count();
    let _ = writeln!(
        stderr,
        "sqlquack audit: {}, {} re-recorded cases, {problems} differ or are invalid",
        if problems == 0 { "done" } else { "FAILED" },
        entries.len()
    );
    if problems == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

pub(crate) fn audit_errors_command(arguments: &AuditArguments, stderr: &mut dyn Write) -> ExitCode {
    let _ = writeln!(
        stderr,
        "sqlquack audit: selecting cases recorded as warehouse errors…"
    );
    match error_pending(arguments) {
        Ok((chosen, written)) => {
            let _ = writeln!(
                stderr,
                "sqlquack audit: done, {chosen} error cases; {written} pending case files in {}",
                arguments.out.display()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack audit: failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn error_pending(arguments: &AuditArguments) -> Result<(usize, usize), String> {
    let evidence = load_evidence(&arguments.paths)?;
    let mut chosen: Vec<LoadedCase> = Vec::new();
    for case in &evidence.cases {
        let stored =
            load_result(&arguments.paths.results, &case.hash).map_err(|error| error.to_string())?;
        if stored.is_some_and(|stored| stored.metadata.outcome == RecordedOutcome::Error) {
            chosen.push(case.clone());
        }
    }
    let version = duckdb_version().map_err(|error| error.to_string())?;
    let summary = write_pending(&chosen, &arguments.audit_results, &arguments.out, &version)
        .map_err(|error| error.to_string())?;
    Ok((chosen.len(), summary.written))
}

fn sample_pending(
    arguments: &AuditArguments,
    percent: f64,
    seed: u64,
) -> Result<(usize, usize), String> {
    let evidence = load_evidence(&arguments.paths)?;
    let chosen: Vec<LoadedCase> =
        sample_cases(&evidence.cases, &arguments.paths.results, percent, seed)
            .into_iter()
            .cloned()
            .collect();
    let version = duckdb_version().map_err(|error| error.to_string())?;
    let summary = write_pending(&chosen, &arguments.audit_results, &arguments.out, &version)
        .map_err(|error| error.to_string())?;
    Ok((chosen.len(), summary.written))
}

fn compare(arguments: &AuditArguments) -> Result<Vec<AuditEntry>, String> {
    let evidence = load_evidence(&arguments.paths)?;
    compare_audit(
        &evidence.cases,
        &arguments.paths.results,
        &arguments.audit_results,
    )
    .map_err(|error| error.to_string())
}

fn render(entries: &[AuditEntry]) -> String {
    let mut rendered = String::new();
    for entry in entries {
        let outcome = match &entry.outcome {
            AuditOutcome::Matched => "matched".to_owned(),
            AuditOutcome::Differed { detail } => format!("DIFFERED: {detail}"),
            AuditOutcome::Invalid { detail } => format!("INVALID: {detail}"),
        };
        rendered.push_str(&format!(
            "{}  {outcome}  (warehouse {} then {})\n",
            entry.reference.as_deref().unwrap_or(&entry.hash),
            entry.stored_warehouse_version.as_deref().unwrap_or("?"),
            entry.fresh_warehouse_version.as_deref().unwrap_or("?")
        ));
    }
    rendered
}
