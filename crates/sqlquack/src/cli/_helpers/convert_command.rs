//! `sqlquack convert`, `check` and `explain`: Snowflake SQL as DuckDB SQL, what is wrong with it, and what each code means.

use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use sqlquack_core::diagnostics::constants::SESSION_PARAMETERS;
use sqlquack_core::diagnostics::main::diagnose::diagnose;
use sqlquack_core::diagnostics::main::explain_code::explain_code;
use sqlquack_core::diagnostics::main::render_diagnostics::render_diagnostics;
use sqlquack_core::diagnostics::main::unplaced_diagnostic::unplaced_diagnostic;
use sqlquack_core::diagnostics::models::Diagnostic;
use sqlquack_core::provenance::main::merge_provenance::merge_provenance;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::layout::main::layout_sql::layout_sql;
use sqlquack_policy::policy::constants::{
    CURRENT_TIME, SESSION_TIME_ZONE, SESSION_TIMEZONE_UTC, TIMEZONE_PARAMETER,
};
use sqlquack_policy::policy::models::{DivergenceFailure, Policy};
use sqlquack_rules::conversion::models::ConversionOptions;
use sqlquack_semantics::types::models::Schema;
use std::collections::{BTreeMap, BTreeSet};

use crate::analysis::main::assumption_id::assumption_id;
use crate::analysis::main::check_source::check_source;
use crate::analysis::main::override_macros::override_macros;
use crate::analysis::main::python_functions::python_functions;
use crate::analysis::main::refused_construct::refused_construct;
use crate::analysis::main::with_overrides::with_overrides;
use crate::analysis::models::Analysis;
use crate::cli::_helpers::evidence::caller_choices;
use crate::cli::models::{
    CheckArguments, CheckReport, ConvertArguments, ConvertRefusal, ConvertReport, ExplainArguments,
    OverrideListing, PolicyArguments,
};

use crate::cli::_helpers::evidence::with_divergence_help;
use crate::cli::main::convert_report::convert_report;

/// The path naming standard input.
const STDIN_PATH: &str = "-";
/// How diagnostics name standard input.
const STDIN_NAME: &str = "<stdin>";

pub(crate) fn convert_command(
    arguments: &ConvertArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let _ = writeln!(stderr, "sqlquack convert: converting…");
    let report = match read_sql(arguments.path.as_deref())
        .and_then(|sql| convert_report(arguments, &sql).map(|report| (report, sql)))
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
        let path = source_name(arguments.path.as_deref());
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

/// The SQL from the file, or from standard input when there is none or it is `-`.
fn read_sql(path: Option<&Path>) -> Result<String, String> {
    match path {
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

/// How diagnostics name the input at `path`.
fn source_name(path: Option<&Path>) -> String {
    match path {
        Some(path) if path.as_os_str() != STDIN_PATH => path.display().to_string(),
        _ => STDIN_NAME.to_owned(),
    }
}

/// `sqlquack check`: exit 0 when the SQL holds no problem, 1 with diagnostics, 2 without SQL.
pub(crate) fn explain_command(
    arguments: &ExplainArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    match explain_code(&arguments.code) {
        Some(text) => {
            let _ = write!(stdout, "{text}");
            ExitCode::SUCCESS
        }
        None => {
            let _ = writeln!(
                stderr,
                "sqlquack explain: no diagnostic code, name or former name is `{}`",
                arguments.code
            );
            ExitCode::from(2)
        }
    }
}

pub(crate) fn check_command(
    arguments: &CheckArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let _ = writeln!(stderr, "sqlquack check: checking…");
    let path = arguments.path.as_deref();
    let (sql, diagnostics) =
        match read_sql(path).and_then(|sql| check_source(&sql).map(|found| (sql, found))) {
            Ok(checked) => checked,
            Err(error) => {
                let _ = writeln!(stderr, "sqlquack check: failed: {error}");
                return ExitCode::from(2);
            }
        };
    let report = CheckReport {
        ok: diagnostics.is_empty(),
        diagnostics,
    };
    if arguments.output.json() {
        let json = serde_json::to_string_pretty(&report).unwrap_or_default() + "\n";
        if stdout.write_all(json.as_bytes()).is_err() {
            return ExitCode::FAILURE;
        }
    }
    let rendered = render_diagnostics(&report.diagnostics, &sql, &source_name(path));
    let _ = write!(stderr, "{rendered}");
    match report.diagnostics.len() {
        0 => {
            let _ = writeln!(stderr, "sqlquack check: done, no problems");
            ExitCode::SUCCESS
        }
        count => {
            let noun = match count {
                1 => "problem",
                _ => "problems",
            };
            let _ = writeln!(stderr, "sqlquack check: found {count} {noun}");
            ExitCode::FAILURE
        }
    }
}

/// The report when the session parameters match no recorded profile.
pub(crate) fn refused_session(reason: String) -> ConvertReport {
    ConvertReport {
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
        divergence_failures: BTreeMap::new(),
        notes: Vec::new(),
        python_functions: Vec::new(),
        macros: Vec::new(),
        overrides: Vec::new(),
    }
}

/// The session's options with what the caller confirms and the user overrides applied.
pub(crate) fn chosen_options(
    arguments: &ConvertArguments,
    policy: Option<&Policy>,
    options: ConversionOptions,
) -> Result<ConversionOptions, String> {
    let options = match policy {
        Some(policy) => caller_choices(options, policy, &arguments.conversion),
        None if arguments.conversion.loud_divergences_only => {
            let mut options = options;
            options
                .divergences
                .retain(|id, _| ![CURRENT_TIME, SESSION_TIMEZONE_UTC].contains(&id.as_str()));
            options
        }
        None => options,
    };
    with_overrides(
        options,
        (
            arguments.overrides.as_deref(),
            &arguments.verified_overrides,
        ),
    )
}

/// The report of a finished analysis of `sql`, its SQL one clause per line unless `compact`.
pub(crate) fn analysis_report(
    analysis: &Analysis,
    (sql, compact): (&str, bool),
    (schema, options, policy): (&Schema, &ConversionOptions, Option<&Policy>),
    (arguments, parameters, profile): (&PolicyArguments, &BTreeMap<String, String>, String),
) -> Result<ConvertReport, String> {
    let converted = analysis.parsed && analysis.refusals.is_empty();
    let provenance = merge_provenance(analysis.conversions.iter().map(|done| &done.provenance));
    let assumptions: Vec<String> = provenance
        .divergences
        .keys()
        .filter(|id| match policy {
            Some(policy) => policy
                .divergences
                .get(assumption_id(id))
                .is_some_and(|divergence| divergence.assumption),
            None => options
                .divergences
                .get(id.as_str())
                .is_some_and(|divergence| divergence.assumption),
        })
        .cloned()
        .collect();
    let divergence_failures = divergence_failures(policy, &provenance.divergences);
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
        .chain(
            provenance
                .divergences
                .keys()
                .filter(|_| policy.is_none())
                .filter_map(|id| options.divergences.get(id.as_str()))
                .map(|divergence| divergence.reason.clone()),
        )
        .collect();
    Ok(ConvertReport {
        converted,
        profile,
        notes,
        sql: match converted {
            true => analysis
                .conversions
                .iter()
                .map(|done| match compact {
                    true => done.sql.clone(),
                    false => layout_sql(&done.sql),
                })
                .collect::<Vec<String>>()
                .join(";\n"),
            false => String::new(),
        },
        refusals: analysis
            .refusals
            .iter()
            .zip(with_divergence_help(
                &analysis.refusals,
                diagnose(&analysis.refusals, sql),
                (policy, arguments, parameters),
            ))
            .map(convert_refusal)
            .collect(),
        python_functions: python_functions(&provenance.divergences, schema)?,
        macros: override_macros(&provenance.divergences, options),
        overrides: override_listings(options),
        divergences: listed(provenance.divergences),
        assumptions,
        divergence_failures,
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

/// Each relied-on divergence's failure, as the policy declares it.
fn divergence_failures(
    policy: Option<&Policy>,
    divergences: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeMap<String, DivergenceFailure> {
    let Some(policy) = policy else {
        return BTreeMap::new();
    };
    let mut failures: BTreeMap<String, DivergenceFailure> = BTreeMap::new();
    for id in divergences.keys() {
        if let Some(divergence) = policy.divergences.get(assumption_id(id)) {
            failures.insert(id.clone(), divergence.failure);
        }
    }
    failures
}

/// Each divergence with its constructs listed.
fn listed(divergences: BTreeMap<String, BTreeSet<String>>) -> BTreeMap<String, Vec<String>> {
    let mut listed: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, constructs) in divergences {
        listed.insert(id, constructs.into_iter().collect());
    }
    listed
}

/// Every user override loaded, as the report lists it.
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
