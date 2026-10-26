//! `sqlquack verify`.

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use sqlquack_evidence::constants::{SESSION_DEFAULTS_FILE, SESSION_DEFAULTS_NAME};
use sqlquack_evidence::results::main::load_session_defaults::load_session_defaults;
use sqlquack_evidence::verify::main::verify_cases::verify_cases;
use sqlquack_evidence::verify::models::{ErrorCheck, Verdict, VerifyInputs, VerifyReport};

use crate::analysis::main::builtin_claims::builtin_claims;
use crate::analysis::main::builtin_failure_modes::builtin_failure_modes;
use crate::analysis::main::check_equivalence_cases::check_equivalence_cases;
use crate::analysis::main::conversion_options::conversion_options;
use crate::analysis::models::PipelineConverter;
use crate::cli::_helpers::evidence::{Evidence, load_evidence};
use crate::cli::models::{EvidencePaths, VerifyArguments};

pub(crate) fn verify_command(
    arguments: &VerifyArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let _ = writeln!(
        stderr,
        "sqlquack verify: loading cases and replaying recorded ones on DuckDB…"
    );
    let report = match verify(arguments) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack verify: failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let rendered = if arguments.json {
        serde_json::to_string_pretty(&report).unwrap_or_default() + "\n"
    } else {
        render(&report)
    };
    let _ = stdout.write_all(rendered.as_bytes());
    let failed = count(&report, |verdict| matches!(verdict, Verdict::Failed { .. }));
    let unverified = report
        .claims
        .iter()
        .filter(|claim| claim.passed_cases == 0)
        .count();
    let error_findings = report
        .cases
        .iter()
        .filter(|case| case.error_check.as_ref().is_some_and(is_finding))
        .count();
    let blocked = failed > 0
        || (arguments.require_verified && (unverified > 0 || report.unproven_sessions > 0))
        || (arguments.strict_errors && error_findings > 0);
    let _ = writeln!(
        stderr,
        "sqlquack verify: error checks: {} agree, {} divergence raises, {error_findings} findings{}",
        count_checks(&report, |check| matches!(check, ErrorCheck::Agrees { .. })),
        count_checks(&report, |check| matches!(
            check,
            ErrorCheck::DivergenceRaise { .. }
        )),
        if arguments.strict_errors {
            ""
        } else {
            " (reported only; --strict-errors fails on them)"
        },
    );
    let _ = writeln!(
        stderr,
        "sqlquack verify: {}, {} cases: {} passed, {} diverged as expected, {} refused, {} typed, \
         {} pending, {failed} failed; {} of {} equivalences and rules verified",
        if blocked { "FAILED" } else { "done" },
        report.cases.len(),
        count(&report, |verdict| *verdict == Verdict::Passed),
        count(&report, |verdict| matches!(
            verdict,
            Verdict::Diverged { .. }
        )),
        count(&report, |verdict| matches!(
            verdict,
            Verdict::Refused { .. }
        )),
        count(&report, |verdict| *verdict == Verdict::Typed),
        count(&report, |verdict| *verdict == Verdict::Pending),
        report.claims.len() - unverified,
        report.claims.len(),
    );
    if report.unproven_sessions > 0 {
        let _ = writeln!(
            stderr,
            "sqlquack verify: {} results were recorded before session parameters were pinned, \
             and no account defaults vouch for them; run `sqlquack-record --session-defaults \
             {SESSION_DEFAULTS_FILE}`{}",
            report.unproven_sessions,
            if arguments.require_verified {
                ""
            } else {
                " (reported only; --require-verified fails on them)"
            },
        );
    }
    if !blocked {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn verify(arguments: &VerifyArguments) -> Result<VerifyReport, String> {
    run_verification(&arguments.paths, Some(&arguments.audit_results)).map(|(_, report)| report)
}

/// Loads the evidence and verifies every case with the production converter.
pub(crate) fn run_verification(
    paths: &EvidencePaths,
    audit_results: Option<&Path>,
) -> Result<(Evidence, VerifyReport), String> {
    let evidence = load_evidence(paths)?;
    let problems = check_equivalence_cases(&evidence.cases);
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    let claims = builtin_claims();
    let failure_modes = builtin_failure_modes();
    let defaults_path = paths.results.parent().map_or_else(
        || Path::new(SESSION_DEFAULTS_FILE).to_owned(),
        |evidence| evidence.join(SESSION_DEFAULTS_NAME),
    );
    let session_defaults =
        load_session_defaults(&defaults_path).map_err(|error| error.to_string())?;
    let report = verify_cases(VerifyInputs {
        cases: &evidence.cases,
        results: &paths.results,
        policy: &evidence.policy,
        converter: &PipelineConverter {
            options: conversion_options(&evidence.policy, false),
        },
        claims: &claims,
        failure_modes: &failure_modes,
        audit_results,
        session_defaults: session_defaults.as_ref(),
    })
    .map_err(|error| error.to_string())?;
    Ok((evidence, report))
}

fn count(report: &VerifyReport, predicate: impl Fn(&Verdict) -> bool) -> usize {
    report
        .cases
        .iter()
        .filter(|case| predicate(&case.verdict))
        .count()
}

fn count_checks(report: &VerifyReport, predicate: impl Fn(&ErrorCheck) -> bool) -> usize {
    report
        .cases
        .iter()
        .filter_map(|case| case.error_check.as_ref())
        .filter(|check| predicate(check))
        .count()
}

/// Whether an error check is a finding: anything but agreement or a divergence's raise.
fn is_finding(check: &ErrorCheck) -> bool {
    !matches!(
        check,
        ErrorCheck::Agrees { .. } | ErrorCheck::DivergenceRaise { .. }
    )
}

fn error_note(check: &ErrorCheck) -> String {
    match check {
        ErrorCheck::Agrees { .. } | ErrorCheck::DivergenceRaise { .. } => String::new(),
        ErrorCheck::UnknownSnowflakeError { errno, sqlstate } => format!(
            "; error check: Snowflake error {} (SQLSTATE {}) is not in the taxonomy",
            errno.unwrap_or_default(),
            sqlstate.as_deref().unwrap_or("unknown")
        ),
        ErrorCheck::ErrnoMismatch { recorded, tagged } => format!(
            "; error check: DuckDB raised Snowflake error {tagged}, but {recorded} is recorded"
        ),
        ErrorCheck::KindMismatch {
            snowflake,
            duckdb,
            error,
        } => format!(
            "; error check: Snowflake raised {snowflake}, DuckDB {}: {error}",
            duckdb.map_or_else(
                || "an error of no known kind".to_owned(),
                |kind| kind.to_string()
            )
        ),
        ErrorCheck::Undeclared { errno, sqlstate } => format!(
            "; error check: no rule used declares Snowflake error {errno} (SQLSTATE {sqlstate})"
        ),
    }
}

fn render(report: &VerifyReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    for case in &report.cases {
        let detail = match &case.verdict {
            Verdict::Pending => "pending: no recorded result yet".to_owned(),
            Verdict::Passed => "passed".to_owned(),
            Verdict::Refused { reasons, .. } => format!(
                "refused: {} reasons, first: {}",
                reasons.len(),
                reasons.first().map_or("", String::as_str)
            ),
            Verdict::Diverged {
                divergence,
                error,
                matched,
            } => {
                format!("diverged as expected ({divergence}, {matched}): DuckDB raised {error}")
            }
            Verdict::Typed => "typed: column types recorded for analysis".to_owned(),
            Verdict::Failed { reason } => format!("FAILED: {reason}"),
        };
        let warnings: String = case
            .provenance
            .iter()
            .flat_map(|provenance| &provenance.divergences)
            .map(|(divergence, constructs)| {
                let constructs: Vec<&str> = constructs.iter().map(String::as_str).collect();
                format!(
                    "; warning: known divergence {divergence} at {}",
                    constructs.join(", ")
                )
            })
            .collect();
        let note = match (
            case.error_check.as_ref().map(error_note),
            &case.snowflake_message,
        ) {
            (Some(note), Some(message)) if !note.is_empty() => {
                format!("{note}; Snowflake said: {message}")
            }
            (note, _) => note.unwrap_or_default(),
        };
        lines.push(format!("{}  {detail}{warnings}{note}", case.reference));
    }
    for claim in report.claims.iter().filter(|claim| claim.passed_cases == 0) {
        lines.push(format!(
            "unverified: {:?} has no passed case yet",
            claim.claim
        ));
    }
    for hash in &report.unreferenced_results {
        lines.push(format!(
            "note: result {hash} is no longer named by any case"
        ));
    }
    lines.join("\n") + "\n"
}
