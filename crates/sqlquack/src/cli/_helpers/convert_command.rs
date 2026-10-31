//! `sqlquack convert`: Snowflake SQL as DuckDB SQL, or the refusals and why.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::process::ExitCode;

use serde::Serialize;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::provenance::main::merge_provenance::merge_provenance;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_policy::policy::constants::{SESSION_TIME_ZONE, TIMEZONE_PARAMETER};
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;

use crate::analysis::main::analyze::analyze;
use crate::analysis::main::parse_session::parse_session;
use crate::analysis::main::refused_construct::refused_construct;
use crate::analysis::main::session_conversion_options::session_conversion_options;
use crate::cli::_helpers::evidence::chosen_policy;
use crate::cli::_helpers::schema::load_schema;
use crate::cli::models::ConvertArguments;

/// The path naming standard input.
const STDIN_PATH: &str = "-";

/// What `convert --json` prints.
#[derive(Serialize)]
struct ConvertReport {
    /// Whether every statement converted.
    converted: bool,
    /// The session profile conversion ran under.
    profile: String,
    /// The DuckDB SQL, one statement after another; empty when refused.
    sql: String,
    /// Each refusal.
    refusals: Vec<ConvertRefusal>,
    /// Known divergences relied on, with the constructs relying on each.
    divergences: BTreeMap<String, Vec<String>>,
    /// Of those, the named assumptions.
    assumptions: Vec<String>,
    /// What the session parameters changed about conversion.
    notes: Vec<String>,
}

/// One refusal as `convert --json` prints it.
#[derive(Serialize)]
struct ConvertRefusal {
    category: String,
    construct: String,
    line: Option<usize>,
    column: Option<usize>,
    reason: String,
}

pub(crate) fn convert_command(
    arguments: &ConvertArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let _ = writeln!(stderr, "sqlquack convert: converting…");
    let report = match converted(arguments) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack convert: failed: {error}");
            return ExitCode::from(2);
        }
    };
    let rendered = match arguments.json {
        true => serde_json::to_string_pretty(&report).unwrap_or_default() + "\n",
        false if report.converted => report.sql.clone() + "\n",
        false => String::new(),
    };
    if stdout.write_all(rendered.as_bytes()).is_err() {
        return ExitCode::FAILURE;
    }
    for note in &report.notes {
        let _ = writeln!(stderr, "sqlquack convert: note: {note}");
    }
    for (divergence, constructs) in &report.divergences {
        let kind = match report.assumptions.contains(divergence) {
            true => "assumption",
            false => "known divergence",
        };
        let _ = writeln!(
            stderr,
            "sqlquack convert: relies on {kind} {divergence} at {}",
            constructs.join(", ")
        );
    }
    for refusal in &report.refusals {
        let at = match (refusal.line, refusal.column) {
            (Some(line), Some(column)) => format!(" at {line}:{column}"),
            _ => String::new(),
        };
        let _ = writeln!(
            stderr,
            "sqlquack convert: refused {}{at} ({}): {}",
            refusal.construct, refusal.category, refusal.reason
        );
    }
    match report.converted {
        true => {
            let _ = writeln!(
                stderr,
                "sqlquack convert: done, converted under {}",
                report.profile
            );
            ExitCode::SUCCESS
        }
        false => {
            let profile = match report.profile.is_empty() {
                true => "no matching session profile".to_owned(),
                false => report.profile.clone(),
            };
            let _ = writeln!(
                stderr,
                "sqlquack convert: refused, {} refusals under {profile}",
                report.refusals.len()
            );
            ExitCode::FAILURE
        }
    }
}

fn converted(arguments: &ConvertArguments) -> Result<ConvertReport, String> {
    let sql = read_sql(arguments)?;
    let schema = arguments
        .schema
        .as_deref()
        .map(load_schema)
        .transpose()?
        .unwrap_or_default();
    let policy = chosen_policy(&arguments.policy, arguments.builtin_policy)?;
    let parameters = parse_session(&arguments.session)?;
    let (options, profile) = match session_conversion_options(
        policy.as_ref(),
        (arguments.strict, arguments.utc_session),
        &parameters,
    ) {
        Ok(matched) => matched,
        Err(reason) => {
            return Ok(ConvertReport {
                converted: false,
                profile: String::new(),
                sql: String::new(),
                refusals: vec![ConvertRefusal {
                    category: "session parameters".to_owned(),
                    construct: "statement".to_owned(),
                    line: None,
                    column: None,
                    reason,
                }],
                divergences: BTreeMap::new(),
                assumptions: Vec::new(),
                notes: Vec::new(),
            });
        }
    };
    let analysis = analyze(
        &sql,
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&schema, &options),
    );
    if analysis.parsed && analysis.statements == 0 {
        return Err("the input holds no SQL statement".to_owned());
    }
    let converted = analysis.parsed && analysis.refusals.is_empty();
    let provenance = merge_provenance(analysis.conversions.iter().map(|done| &done.provenance));
    let assumptions: Vec<String> = provenance
        .divergences
        .keys()
        .filter(|id| {
            policy
                .as_ref()
                .and_then(|policy| policy.divergences.get(*id))
                .is_some_and(|divergence| divergence.assumption)
        })
        .cloned()
        .collect();
    let notes: Vec<String> = parameters
        .get(TIMEZONE_PARAMETER)
        .filter(|zone| *zone != SESSION_TIME_ZONE)
        .map(|zone| {
            format!(
                "TIMEZONE = {zone} is not UTC, so statements depending on the session time zone \
                 are refused, --utc-session or not"
            )
        })
        .into_iter()
        .collect();
    Ok(ConvertReport {
        converted,
        profile,
        notes,
        sql: match converted {
            true => analysis
                .conversions
                .iter()
                .map(|done| done.sql.clone())
                .collect::<Vec<String>>()
                .join(";\n"),
            false => String::new(),
        },
        refusals: analysis.refusals.iter().map(convert_refusal).collect(),
        divergences: listed(provenance.divergences),
        assumptions,
    })
}

/// A refusal as `convert` reports it.
fn convert_refusal(refusal: &Refusal) -> ConvertRefusal {
    ConvertRefusal {
        category: refusal.category.label().to_owned(),
        construct: refused_construct(refusal),
        line: refusal.location.map(|location| location.line),
        column: refusal.location.map(|location| location.column),
        reason: refusal.reason.clone(),
    }
}

/// Each divergence with its constructs listed.
fn listed(divergences: BTreeMap<String, BTreeSet<String>>) -> BTreeMap<String, Vec<String>> {
    let mut listed: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, constructs) in divergences {
        listed.insert(id, constructs.into_iter().collect());
    }
    listed
}

/// The SQL from the file, or from standard input when there is none or it is `-`.
fn read_sql(arguments: &ConvertArguments) -> Result<String, String> {
    match arguments.path.as_deref() {
        Some(path) if path.as_os_str() != STDIN_PATH => std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display())),
        _ => {
            let mut sql = String::new();
            std::io::stdin()
                .read_to_string(&mut sql)
                .map_err(|error| format!("cannot read standard input: {error}"))?;
            Ok(sql)
        }
    }
}
