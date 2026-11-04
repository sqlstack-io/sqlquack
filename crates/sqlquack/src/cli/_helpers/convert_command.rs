//! `sqlquack convert`: Snowflake SQL as DuckDB SQL, or the refusals and why.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::process::ExitCode;

use serde::Serialize;
use sqlquack_core::diagnostics::constants::SESSION_PARAMETERS;
use sqlquack_core::diagnostics::main::diagnose::diagnose;
use sqlquack_core::diagnostics::main::render_diagnostics::render_diagnostics;
use sqlquack_core::diagnostics::main::unplaced_diagnostic::unplaced_diagnostic;
use sqlquack_core::diagnostics::models::Diagnostic;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::provenance::main::merge_provenance::merge_provenance;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_policy::policy::constants::{SESSION_TIME_ZONE, TIMEZONE_PARAMETER};
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_rules::conversion::models::ConversionOptions;

use crate::analysis::main::analyze::analyze;
use crate::analysis::main::assumption_id::assumption_id;
use crate::analysis::main::override_macros::override_macros;
use crate::analysis::main::parse_session::parse_session;
use crate::analysis::main::python_functions::python_functions;
use crate::analysis::main::refused_construct::refused_construct;
use crate::analysis::main::session_conversion_options::session_conversion_options;
use crate::analysis::main::with_overrides::with_overrides;
use crate::analysis::models::PythonSignature;
use crate::cli::_helpers::evidence::chosen_policy;
use crate::cli::_helpers::evidence::confirmed_offsets;
use crate::cli::_helpers::schema::load_schema;
use crate::cli::models::ConvertArguments;

/// The path naming standard input.
const STDIN_PATH: &str = "-";
/// How diagnostics name standard input.
const STDIN_NAME: &str = "<stdin>";

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
    /// The Python functions the SQL calls, which the caller registers before running it.
    python_functions: Vec<PythonSignature>,
    /// The macros of the user overrides the SQL calls, which the caller runs before it.
    macros: Vec<String>,
    /// Every user override loaded: its name, the function it replaces and its body's hash.
    overrides: Vec<OverrideListing>,
}

#[derive(Serialize)]
struct OverrideListing {
    name: String,
    replaces: String,
    sha256: String,
}

/// One refusal as `convert --json` prints it: the original fields, then its diagnostic's.
#[derive(Serialize)]
struct ConvertRefusal {
    category: String,
    construct: String,
    line: Option<usize>,
    column: Option<usize>,
    reason: String,
    #[serde(flatten)]
    diagnostic: Option<Diagnostic>,
}

pub(crate) fn convert_command(
    arguments: &ConvertArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let _ = writeln!(stderr, "sqlquack convert: converting…");
    let report = match read_sql(arguments)
        .and_then(|sql| converted(arguments, &sql).map(|report| (report, sql)))
    {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack convert: failed: {error}");
            return ExitCode::from(2);
        }
    };
    let (report, sql) = report;
    let rendered = match arguments.output.json() {
        true => serde_json::to_string_pretty(&report).unwrap_or_default() + "\n",
        false if report.converted => {
            report
                .macros
                .iter()
                .map(|statement| format!("{statement};\n"))
                .collect::<String>()
                + &report.sql
                + "\n"
        }
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
    let diagnostics: Vec<Diagnostic> = report
        .refusals
        .iter()
        .filter_map(|refusal| refusal.diagnostic.clone())
        .collect();
    if !diagnostics.is_empty() {
        let path = match arguments.path.as_deref() {
            Some(path) if path.as_os_str() != STDIN_PATH => path.display().to_string(),
            _ => STDIN_NAME.to_owned(),
        };
        let _ = write!(stderr, "{}", render_diagnostics(&diagnostics, &sql, &path));
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
            let count = report.refusals.len();
            let noun = match count {
                1 => "refusal",
                _ => "refusals",
            };
            let _ = writeln!(
                stderr,
                "sqlquack convert: refused, {count} {noun} under {profile}"
            );
            ExitCode::FAILURE
        }
    }
}

fn converted(arguments: &ConvertArguments, sql: &str) -> Result<ConvertReport, String> {
    let schema = arguments
        .schema
        .as_deref()
        .map(load_schema)
        .transpose()?
        .unwrap_or_default();
    let policy = chosen_policy(
        &arguments.conversion.policy,
        arguments.conversion.builtin_policy,
    )?;
    let parameters = parse_session(&arguments.session)?;
    let (options, profile) = match session_conversion_options(
        policy.as_ref(),
        (
            arguments.conversion.strict,
            arguments.conversion.utc_session,
        ),
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
                    diagnostic: unplaced_diagnostic(SESSION_PARAMETERS, reason.clone()),
                    reason,
                }],
                divergences: BTreeMap::new(),
                assumptions: Vec::new(),
                notes: Vec::new(),
                python_functions: Vec::new(),
                macros: Vec::new(),
                overrides: Vec::new(),
            });
        }
    };
    let options = match &policy {
        Some(policy) => confirmed_offsets(
            options,
            policy,
            (
                arguments.conversion.confirm_declared_offsets,
                arguments.conversion.python_runtime,
            ),
        ),
        None => options,
    };
    let options = with_overrides(
        options,
        (
            arguments.overrides.as_deref(),
            &arguments.verified_overrides,
        ),
    )?;
    let analysis = analyze(
        sql,
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
                .and_then(|policy| policy.divergences.get(assumption_id(id)))
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
        refusals: analysis
            .refusals
            .iter()
            .zip(diagnose(&analysis.refusals, sql))
            .map(convert_refusal)
            .collect(),
        python_functions: python_functions(&provenance.divergences, &schema)?,
        macros: override_macros(&provenance.divergences, &options),
        overrides: override_listings(&options),
        divergences: listed(provenance.divergences),
        assumptions,
    })
}

/// A refusal as `convert` reports it, with its diagnostic.
fn convert_refusal((refusal, diagnostic): (&Refusal, Diagnostic)) -> ConvertRefusal {
    ConvertRefusal {
        diagnostic: Some(diagnostic),
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

fn override_listings(options: &ConversionOptions) -> Vec<OverrideListing> {
    options
        .overrides
        .iter()
        .map(|(replaces, replacement)| OverrideListing {
            name: replacement.name.clone(),
            replaces: replaces.clone(),
            sha256: replacement.sha256.clone(),
        })
        .collect()
}
