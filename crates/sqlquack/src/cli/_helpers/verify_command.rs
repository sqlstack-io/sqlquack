//! `sqlquack verify` and `sqlquack compare`.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sqlquack_blocks::macros::main::helper_statements::helper_statements;
use sqlquack_compare::comparison::models::RowOrder;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_evidence::constants::{SESSION_DEFAULTS_FILE, SESSION_DEFAULTS_NAME};
use sqlquack_evidence::coverage::constants::COVERAGE_GAPS_LISTED;
use sqlquack_evidence::coverage::main::check_coverage::check_coverage;
use sqlquack_evidence::coverage::main::load_domains::load_domains;
use sqlquack_evidence::coverage::main::load_hazards::load_hazards;
use sqlquack_evidence::coverage::models::CoverageReport;
use sqlquack_evidence::replay::main::replay_on_engine::replay_on_engine;
use sqlquack_evidence::replay::models::ReplaySettings;
use sqlquack_evidence::results::main::load_recording::load_recording;
use sqlquack_evidence::results::main::load_session_defaults::load_session_defaults;
use sqlquack_evidence::verify::main::judge_recording::judge_recording;
use sqlquack_evidence::verify::main::verify_cases::verify_cases;
use sqlquack_evidence::verify::models::RecordingMatch;
use sqlquack_evidence::verify::models::{
    CaseVerdict, Conversion, ErrorCheck, Verdict, VerifyInputs, VerifyReport,
};
use sqlquack_evidence::verify::types::CaseConverter;
use sqlquack_policy::policy::constants::{HASH_DIVERGENCE, PROJECT_UDF};
use sqlquack_policy::policy::main::load_policy::load_policy;
use sqlquack_policy::policy::models::Policy;
use sqlquack_rules::catalogue::constants::RULES;
use sqlquack_rules::conversion::models::ConversionOptions;

use crate::analysis::main::builtin_claims::builtin_claims;
use crate::analysis::main::builtin_failure_modes::builtin_failure_modes;
use crate::analysis::main::check_equivalence_cases::check_equivalence_cases;
use crate::analysis::main::confirm_assumption::confirm_assumption;
use crate::analysis::models::PipelineConverter;
use crate::cli::_helpers::evidence::{Evidence, load_evidence};
use crate::cli::models::{
    CompareArguments, Comparison, CoverageArguments, EvidencePaths, VerifyArguments,
};
use crate::statements::main::write_sql_sizes::write_sql_sizes;
use crate::statements::main::write_translation_statements::write_translation_statements;
use sqlquack_rules::session::main::conversion_options::conversion_options;

/// The search path over read-only inputs: their tables, then the helper macros kept in memory.
const HELPERS_BESIDE_INPUTS: &str = "SET search_path = 'inputs.main,memory.sqlquack'";

pub(crate) fn verify_command(
    arguments: &VerifyArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let _ = writeln!(
        stderr,
        "sqlquack verify: loading cases and replaying recorded ones on DuckDB…"
    );
    let (evidence, report) = match verify(arguments) {
        Ok(verified) => verified,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack verify: failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let coverage = coverage(&arguments.coverage, &evidence, &report);
    let rendered = if arguments.output.json() {
        serde_json::to_string_pretty(&report).unwrap_or_default() + "\n"
    } else {
        render(&report) + &coverage.as_ref().map(render_coverage).unwrap_or_default()
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
    let coverage_blocked = arguments.coverage.require_coverage
        && !arguments.coverage.coverage_projected
        && match &coverage {
            Ok(coverage) => coverage.totals.has_gaps(),
            Err(_) => true,
        };
    for line in report_coverage(&arguments.coverage, coverage.as_ref()) {
        let _ = writeln!(stderr, "sqlquack verify: {line}");
    }
    let unwritten = match &arguments.translation_statements {
        Some(path) => {
            let converter = python_converter(&evidence.policy, arguments.python.as_deref());
            match write_translation_statements(path, &evidence.cases, &report, &converter) {
                Ok(count) => {
                    let _ = writeln!(
                        stderr,
                        "sqlquack verify: wrote {count} statements to {}",
                        path.display()
                    );
                    false
                }
                Err(error) => {
                    let _ = writeln!(stderr, "sqlquack verify: translation statements: {error}");
                    true
                }
            }
        }
        None => false,
    };
    let unwritten = unwritten
        || match &arguments.sql_sizes {
            Some(path) => {
                let converter = python_converter(&evidence.policy, arguments.python.as_deref());
                match write_sql_sizes(path, &evidence.cases, &converter) {
                    Ok(count) => {
                        let _ = writeln!(
                            stderr,
                            "sqlquack verify: wrote the SQL sizes of {count} cases to {}",
                            path.display()
                        );
                        false
                    }
                    Err(error) => {
                        let _ = writeln!(stderr, "sqlquack verify: SQL sizes: {error}");
                        true
                    }
                }
            }
            None => false,
        };
    let blocked = unwritten
        || failed > 0
        || (arguments.require_verified && (unverified > 0 || report.unproven_sessions > 0))
        || (arguments.strict_errors && error_findings > 0)
        || coverage_blocked;
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
         {} pending, {} unproven, {failed} failed; {} of {} equivalences and rules verified",
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
        count(&report, |verdict| matches!(
            verdict,
            Verdict::Unproven { .. }
        )),
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

fn verify(arguments: &VerifyArguments) -> Result<(Evidence, VerifyReport), String> {
    run_verification(
        &arguments.paths,
        Some(&arguments.audit_results),
        arguments.python.as_deref(),
    )
}

/// The coverage report, or why it could not be made.
fn coverage(
    arguments: &CoverageArguments,
    evidence: &Evidence,
    report: &VerifyReport,
) -> Result<CoverageReport, String> {
    let domains = load_domains(&arguments.domains).map_err(|error| error.to_string())?;
    let catalogue = load_hazards(&arguments.hazards).map_err(|error| error.to_string())?;
    let rules: Vec<&str> = RULES.iter().map(|rule| rule.id).collect();
    let (verdicts, projected_refusals) = match arguments.coverage_projected {
        true => projected(evidence, report),
        false => (report.cases.clone(), Vec::new()),
    };
    Ok(CoverageReport {
        projected_refusals,
        ..check_coverage(&rules, (&domains, &catalogue), (&evidence.cases, &verdicts))
    })
}

/// The verdicts with pending cases that convert counted as passed, and the pending cases refused.
fn projected(evidence: &Evidence, report: &VerifyReport) -> (Vec<CaseVerdict>, Vec<String>) {
    let converter = PipelineConverter {
        options: verified_options(&evidence.policy),
        python: None,
    };
    let cases: BTreeMap<&str, &LoadedCase> = evidence
        .cases
        .iter()
        .map(|case| (case.reference.as_str(), case))
        .collect();
    let mut refused: Vec<String> = Vec::new();
    let mut verdicts: Vec<CaseVerdict> = Vec::new();
    for verdict in &report.cases {
        let converted = match (&verdict.verdict, cases.get(verdict.reference.as_str())) {
            (Verdict::Pending, Some(case)) => match converter.convert(case) {
                Conversion::Converted { provenance, .. } => CaseVerdict {
                    verdict: Verdict::Passed,
                    provenance: Some(provenance),
                    ..verdict.clone()
                },
                Conversion::Refused { reasons, .. } => {
                    refused.push(format!(
                        "{}: {}",
                        verdict.reference,
                        reasons.first().map_or("", String::as_str)
                    ));
                    verdict.clone()
                }
            },
            _ => verdict.clone(),
        };
        verdicts.push(converted);
    }
    (verdicts, refused)
}

/// The coverage totals to print, after writing the JSON report when asked.
fn report_coverage(
    arguments: &CoverageArguments,
    coverage: Result<&CoverageReport, &String>,
) -> Vec<String> {
    let projected = match arguments.coverage_projected {
        true => ", projected: pending cases that convert counted as passed",
        false => "",
    };
    let mode = match arguments.require_coverage {
        true => "",
        false => " (reported only; --require-coverage fails on them)",
    };
    let coverage = match coverage {
        Ok(coverage) => coverage,
        Err(error) => return vec![format!("coverage not checked: {error}{mode}")],
    };
    let totals = &coverage.totals;
    let mut lines = vec![format!(
        "coverage (hazard catalogue v{}{projected}): {} of {} rules declare a domain, {} \
         undeclarable; {} of {} combinations without a passing case, {} shapes accepted outside a \
         declared domain; {} of {} hazards without a passing case, {} unrecordable{mode}",
        coverage.catalogue_version,
        totals.rules - totals.undeclared_rules,
        totals.rules,
        totals.undeclarable_rules,
        totals.shape_gaps,
        totals.combinations,
        totals.undeclared_shapes,
        totals.hazard_gaps,
        totals.hazards,
        totals.unrecordable_hazards,
    )];
    if arguments.coverage_projected {
        lines.push(format!(
            "projected coverage: {} pending cases are refused and would cover nothing",
            coverage.projected_refusals.len()
        ));
    }
    if let Some(path) = &arguments.coverage_json {
        let text = serde_json::to_string_pretty(coverage).unwrap_or_default() + "\n";
        if let Err(error) = std::fs::write(path, text) {
            lines.push(format!("cannot write the coverage report: {error}"));
        }
    }
    lines
}

/// One line per rule with a gap, and the first of each kind of gap.
fn render_coverage(coverage: &CoverageReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    for rule in &coverage.rules {
        let status = match (rule.declared, &rule.undeclarable) {
            (false, _) => "no declared domain".to_owned(),
            (true, Some(reason)) => format!("domain cannot be stated: {reason}"),
            (true, None) => format!(
                "{} of {} combinations and {} of {} hazards without a passing case, {} shapes \
                 outside the domain",
                rule.shape_gaps.len(),
                rule.combinations,
                rule.hazard_gaps.len(),
                rule.hazards,
                rule.undeclared_shapes.len()
            ),
        };
        let gaps = rule.shape_gaps.len() + rule.hazard_gaps.len() + rule.undeclared_shapes.len();
        if rule.declared && rule.undeclarable.is_none() && gaps == 0 {
            continue;
        }
        lines.push(format!("coverage: {}: {status}", rule.rule));
        let listed = [
            ("accepted outside the domain:", &rule.undeclared_shapes),
            ("combination", &rule.shape_gaps),
            ("hazard", &rule.hazard_gaps),
        ];
        for (label, gaps) in listed {
            for gap in gaps.iter().take(COVERAGE_GAPS_LISTED) {
                lines.push(format!("coverage:   {label} {gap}"));
            }
            if gaps.len() > COVERAGE_GAPS_LISTED {
                let more = gaps.len() - COVERAGE_GAPS_LISTED;
                lines.push(format!("coverage:   … {more} more (see --coverage-json)"));
            }
        }
    }
    lines.iter().map(|line| format!("{line}\n")).collect()
}

/// Loads the evidence and verifies every case with the production converter.
pub(crate) fn run_verification(
    paths: &EvidencePaths,
    audit_results: Option<&Path>,
    python: Option<&Path>,
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
        converter: &python_converter(&evidence.policy, python),
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
                ..
            } => {
                format!("diverged as expected ({divergence}, {matched}): DuckDB raised {error}")
            }
            Verdict::Typed => "typed: column types recorded for analysis".to_owned(),
            Verdict::Unproven { reason } => format!("unproven: {reason}"),
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

/// `policy`'s default mode with HASH allowed: its recorded cases check the opt-in conversion.
fn verified_options(policy: &Policy) -> ConversionOptions {
    confirm_assumption(conversion_options(policy, false), policy, HASH_DIVERGENCE)
}

/// The converter `verified_options` gives, relying on `project_udf` when Python replays cases.
pub(crate) fn python_converter(policy: &Policy, python: Option<&Path>) -> PipelineConverter {
    let options = verified_options(policy);
    PipelineConverter {
        options: match python {
            Some(_) => confirm_assumption(options, policy, PROJECT_UDF),
            None => options,
        },
        python: python.map(Path::to_path_buf),
    }
}

/// `sqlquack compare`: each manifest entry's converted SQL replayed and judged on DuckDB.
pub(crate) fn compare_command(
    arguments: &CompareArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    match compare(arguments) {
        Ok(outcomes) => {
            let _ = writeln!(stdout, "{}", serde_json::Value::Array(outcomes));
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack compare: failed: {error}");
            ExitCode::from(2)
        }
    }
}

fn compare(arguments: &CompareArguments) -> Result<Vec<serde_json::Value>, String> {
    let policy = load_policy(&arguments.policy).map_err(|error| error.to_string())?;
    let profile = policy
        .session_profiles
        .get(&arguments.session_profile)
        .ok_or_else(|| {
            format!(
                "the policy has no session profile `{}`",
                arguments.session_profile
            )
        })?;
    let settings = ReplaySettings {
        time_zone: profile.duckdb_time_zone.clone(),
        null_order: policy.comparator.null_order.duckdb_setting().to_owned(),
    };
    let text = std::fs::read_to_string(&arguments.manifest).map_err(|error| error.to_string())?;
    let comparisons: Vec<Comparison> =
        serde_json::from_str(&text).map_err(|error| error.to_string())?;
    Ok(comparisons
        .par_iter()
        .map(|comparison| {
            let (outcome, detail) = judged(comparison, &policy, &settings);
            serde_json::json!({"id": comparison.id, "outcome": outcome, "detail": detail})
        })
        .collect())
}

/// The outcome of one comparison and what lies behind it.
fn judged(
    comparison: &Comparison,
    policy: &Policy,
    settings: &ReplaySettings,
) -> (&'static str, String) {
    let recorded = match load_recording(&comparison.recorded) {
        Ok(recorded) => recorded,
        Err(error) => return ("unreadable", error.to_string()),
    };
    let sql = match std::fs::read_to_string(&comparison.sql) {
        Ok(sql) => sql,
        Err(error) => {
            return (
                "unreadable",
                format!("{}: {error}", comparison.sql.display()),
            );
        }
    };
    let database = comparison
        .database
        .display()
        .to_string()
        .replace('\'', "''");
    let attach = format!("ATTACH '{database}' AS inputs (READ_ONLY); USE inputs;");
    let helpers = helper_statements(&[&sql]);
    let setup: Vec<String> = match helpers.is_empty() {
        true => vec![attach],
        false => helpers
            .into_iter()
            .chain([attach, HELPERS_BESIDE_INPUTS.to_owned()])
            .collect(),
    };
    let replayed = match replay_on_engine((&setup, &sql), None, settings) {
        Ok(replayed) => replayed,
        Err(error) => return ("duckdb_error", error),
    };
    let order = match comparison.ordered {
        true => RowOrder::Ordered,
        false => RowOrder::Bag,
    };
    match judge_recording(&recorded, &replayed, order, &policy.comparator) {
        Ok(RecordingMatch::Matches) => ("matches", String::new()),
        Ok(RecordingMatch::MatchesAtMicroseconds) => ("matches_at_microseconds", String::new()),
        Ok(RecordingMatch::Differs(mismatch)) => {
            ("differs", format!("{:?}: {mismatch}", mismatch.kind))
        }
        Err(error) => ("differs", error),
    }
}
