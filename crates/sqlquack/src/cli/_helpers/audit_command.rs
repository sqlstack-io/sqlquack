//! `sqlquack audit`.

use std::io::Write;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlquack_evidence::audit::main::compare_audit::compare_audit;
use sqlquack_evidence::audit::main::sample_cases::sample_cases;
use sqlquack_evidence::audit::models::AuditOutcome;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_evidence::pending::main::write_pending::write_pending;
use sqlquack_evidence::replay::main::duckdb_version::duckdb_version;

use crate::cli::_helpers::evidence::{Evidence, load_evidence};
use crate::cli::models::AuditArguments;

pub(crate) fn audit_command(
    arguments: &AuditArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let Some(evidence) = load_evidence("audit", &arguments.paths, stderr) else {
        return ExitCode::FAILURE;
    };
    match &arguments.sample {
        Some(sample) => sample_command(arguments, sample, &evidence, stderr),
        None => compare_command(arguments, &evidence, stdout, stderr),
    }
}

fn sample_command(
    arguments: &AuditArguments,
    sample: &str,
    evidence: &Evidence,
    stderr: &mut dyn Write,
) -> ExitCode {
    let Some(percent) = sample
        .trim()
        .strip_suffix('%')
        .and_then(|number| number.trim().parse::<f64>().ok())
        .filter(|percent| (0.0..=100.0).contains(percent))
    else {
        let _ = writeln!(
            stderr,
            "sqlquack audit: --sample needs a percentage such as 10%"
        );
        return ExitCode::from(2);
    };
    let seed = arguments.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs())
    });
    let chosen: Vec<LoadedCase> =
        sample_cases(&evidence.cases, &arguments.paths.results, percent, seed)
            .into_iter()
            .cloned()
            .collect();
    let written = duckdb_version().and_then(|version| {
        write_pending(&chosen, &arguments.audit_results, &arguments.out, &version)
    });
    match written {
        Ok(summary) => {
            let _ = writeln!(
                stderr,
                "sqlquack audit: done, sampled {} recorded cases with seed {seed}; {} pending case files in {}",
                chosen.len(),
                summary.written,
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

fn compare_command(
    arguments: &AuditArguments,
    evidence: &Evidence,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let entries = match compare_audit(
        &evidence.cases,
        &arguments.paths.results,
        &arguments.audit_results,
    ) {
        Ok(entries) => entries,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack audit: failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let rendered = if arguments.json {
        serde_json::to_string_pretty(&entries).unwrap_or_default() + "\n"
    } else {
        entries
            .iter()
            .map(|entry| {
                let outcome = match &entry.outcome {
                    AuditOutcome::Matched => "matched".to_owned(),
                    AuditOutcome::Differed { detail } => format!("DIFFERED: {detail}"),
                    AuditOutcome::Invalid { detail } => format!("INVALID: {detail}"),
                };
                format!(
                    "{}  {outcome}  (warehouse {} then {})\n",
                    entry.reference.as_deref().unwrap_or(&entry.hash),
                    entry.stored_warehouse_version.as_deref().unwrap_or("?"),
                    entry.fresh_warehouse_version.as_deref().unwrap_or("?")
                )
            })
            .collect()
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
