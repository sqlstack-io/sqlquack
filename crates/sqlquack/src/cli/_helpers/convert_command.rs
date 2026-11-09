//! `sqlquack convert`, `check`, `explain` and `run`: DuckDB SQL for Snowflake SQL, its problems, its codes, and its rows.

use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use sqlquack_blocks::layout::main::layout_sql::layout_sql;
use sqlquack_diagnostics::diagnostics::constants::SESSION_PARAMETERS;
use sqlquack_diagnostics::diagnostics::main::diagnose::diagnose;
use sqlquack_diagnostics::diagnostics::main::explain_code::explain_code;
use sqlquack_diagnostics::diagnostics::main::render_styled_diagnostics::render_styled_diagnostics;
use sqlquack_diagnostics::diagnostics::main::unplaced_diagnostic::unplaced_diagnostic;
use sqlquack_diagnostics::diagnostics::models::{Diagnostic, Palette};
use sqlquack_diagnostics::provenance::main::merge_provenance::merge_provenance;
use sqlquack_diagnostics::refusals::models::Refusal;
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
use crate::cli::_helpers::evidence::caller_choices;
use crate::cli::_helpers::project::project_choices;
use crate::cli::models::{
    CheckArguments, CheckReport, ColorArguments, ConvertArguments, ConvertRefusal, ConvertReport,
    ExplainArguments, OutputArguments, OutputFormat, OverrideListing, PolicyArguments,
    RunArguments, RunPrinted,
};
use crate::run::main::run_query::run_query;
use crate::run::models::RunInputs;
use crate::terminal::main::highlight_sql::highlight_sql;
use sqlquack_convert::analysis::models::Analysis;

use crate::cli::_helpers::evidence::with_divergence_help;
use crate::cli::main::convert_report::convert_report;

/// The path naming standard input.
const STDIN_PATH: &str = "-";
/// How diagnostics name standard input.
const STDIN_NAME: &str = "<stdin>";

pub(crate) fn convert_command(
    arguments: &ConvertArguments,
    (stdout, stderr): (&mut dyn Write, &mut dyn Write),
    (sql_palette, palette): (&Palette, &Palette),
) -> ExitCode {
    let _ = writeln!(stderr, "sqlquack convert: converting…");
    let report = match read_sql(arguments.path.as_deref())
        .and_then(|sql| convert_report(arguments, &sql).map(|report| (report, sql)))
    {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "sqlquack convert: {}failed{}: {error}",
                palette.error, palette.reset
            );
            return ExitCode::from(2);
        }
    };
    let (report, sql) = report;
    let rendered = match arguments.output.json() {
        true => serde_json::to_string_pretty(&report).unwrap_or_default() + "\n",
        false if report.converted => highlight_sql(
            &(report
                .macros
                .iter()
                .map(|statement| format!("{statement};\n"))
                .collect::<String>()
                + &report.sql
                + "\n"),
            sql_palette,
        ),
        false => String::new(),
    };
    if stdout.write_all(rendered.as_bytes()).is_err() {
        return ExitCode::FAILURE;
    }
    let findings = conversion_findings(
        "convert",
        &report,
        (&sql, arguments.path.as_deref()),
        palette,
    );
    let _ = stderr.write_all(findings.as_bytes());
    match report.converted {
        true => {
            let _ = writeln!(
                stderr,
                "sqlquack convert: {}done, converted{} under {}",
                palette.success, palette.reset, report.profile
            );
            ExitCode::SUCCESS
        }
        false => ExitCode::FAILURE,
    }
}

/// `sqlquack run`: exit 0 when the query ran as expected, 1 refused or different, 2 when it cannot.
pub(crate) fn fixture_run_command(
    arguments: &RunArguments,
    (stdout, stderr): (&mut dyn Write, &mut dyn Write),
    palette: &Palette,
) -> ExitCode {
    let printed = run_printed(arguments, palette);
    let _ = stderr.write_all(printed.findings.as_bytes());
    if stdout.write_all(printed.table.as_bytes()).is_err() {
        return ExitCode::FAILURE;
    }
    let _ = stderr.write_all(printed.verdict.as_bytes());
    printed.exit
}

/// What `run` prints for `arguments`, and how it exits.
fn run_printed(arguments: &RunArguments, palette: &Palette) -> RunPrinted {
    let failed = |findings: String, error: String| RunPrinted {
        exit: ExitCode::from(2),
        findings,
        table: String::new(),
        verdict: format!("sqlquack run: failed: {error}\n"),
    };
    let convert = ConvertArguments {
        path: arguments.path.clone(),
        schema: None,
        overrides: arguments.overrides.clone(),
        verified_overrides: arguments.verified_overrides.clone(),
        session: arguments.session.clone(),
        compact: true,
        output: OutputArguments {
            format: OutputFormat::Text,
            json: false,
        },
        colors: ColorArguments::default(),
        conversion: arguments.conversion.clone(),
    };
    let (sql, report) = match read_sql(arguments.path.as_deref())
        .and_then(|sql| convert_report(&convert, &sql).map(|report| (sql, report)))
    {
        Ok(converted) => converted,
        Err(error) => return failed(String::new(), error),
    };
    let findings = conversion_findings("run", &report, (&sql, arguments.path.as_deref()), palette);
    if !report.converted {
        return RunPrinted {
            exit: ExitCode::FAILURE,
            findings,
            table: String::new(),
            verdict: String::new(),
        };
    }
    if !report.python_functions.is_empty() {
        let error = "the query calls Python functions, which run cannot execute".to_owned();
        return failed(findings, error);
    }
    let ran = project_choices(
        &arguments.conversion,
        (None, &arguments.session, arguments.overrides.as_deref()),
    )
    .and_then(|(policy, project)| {
        run_query(&RunInputs {
            source: &sql,
            converted: &report.sql,
            macros: &report.macros,
            profile: &report.profile,
            policy: policy.as_ref(),
            schema: &project.schema,
            fixtures: &arguments.fixtures,
            expect: arguments.expect.as_deref(),
        })
    });
    let outcome = match ran {
        Ok(outcome) => outcome,
        Err(error) => return failed(findings, error),
    };
    let count = match outcome.rows {
        1 => "1 row".to_owned(),
        count => format!("{count} rows"),
    };
    let (exit, verdict) = match outcome.differences {
        None => (ExitCode::SUCCESS, format!("sqlquack run: done, {count}\n")),
        Some(differences) if differences.is_empty() => (
            ExitCode::SUCCESS,
            format!("sqlquack run: done, {count} as expected\n"),
        ),
        Some(differences) => (
            ExitCode::FAILURE,
            differences.iter().fold(
                "sqlquack run: the result differs from --expect\n".to_owned(),
                |text, difference| text + "  " + difference + "\n",
            ),
        ),
    };
    RunPrinted {
        exit,
        findings,
        table: outcome.table,
        verdict,
    }
}

/// What `command` noticed converting `sql`, and its refusals when it was refused, as text.
fn conversion_findings(
    command: &str,
    report: &ConvertReport,
    (sql, path): (&str, Option<&Path>),
    palette: &Palette,
) -> String {
    let mut text = String::new();
    for warning in &report.warnings {
        text.push_str(&format!("sqlquack {command}: warning: {warning}\n"));
    }
    for note in &report.notes {
        text.push_str(&format!("sqlquack {command}: note: {note}\n"));
    }
    for (divergence, constructs) in &report.divergences {
        let kind = match report.assumptions.contains(divergence) {
            true => "assumption",
            false => "known divergence",
        };
        text.push_str(&format!(
            "sqlquack {command}: relies on {kind} {divergence} at {}\n",
            constructs.join(", ")
        ));
    }
    let diagnostics: Vec<Diagnostic> = report
        .refusals
        .iter()
        .filter_map(|refusal| refusal.diagnostic.clone())
        .collect();
    if !diagnostics.is_empty() {
        text.push_str(&render_styled_diagnostics(
            &diagnostics,
            sql,
            &source_name(path),
            palette,
        ));
    }
    if !report.converted {
        let profile = match report.profile.is_empty() {
            true => "no matching session profile".to_owned(),
            false => report.profile.clone(),
        };
        let noun = match report.refusals.len() {
            1 => "refusal",
            _ => "refusals",
        };
        text.push_str(&format!(
            "sqlquack {command}: {}refused, {} {noun}{} under {profile}\n",
            palette.error,
            report.refusals.len(),
            palette.reset
        ));
    }
    text
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
    palette: &Palette,
) -> ExitCode {
    let _ = writeln!(stderr, "sqlquack check: checking…");
    let path = arguments.path.as_deref();
    let (sql, diagnostics) =
        match read_sql(path).and_then(|sql| check_source(&sql).map(|found| (sql, found))) {
            Ok(checked) => checked,
            Err(error) => {
                let _ = writeln!(
                    stderr,
                    "sqlquack check: {}failed{}: {error}",
                    palette.error, palette.reset
                );
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
    let rendered =
        render_styled_diagnostics(&report.diagnostics, &sql, &source_name(path), palette);
    let _ = write!(stderr, "{rendered}");
    match report.diagnostics.len() {
        0 => {
            let _ = writeln!(
                stderr,
                "sqlquack check: {}done, no problems{}",
                palette.success, palette.reset
            );
            ExitCode::SUCCESS
        }
        count => {
            let noun = match count {
                1 => "problem",
                _ => "problems",
            };
            let _ = writeln!(
                stderr,
                "sqlquack check: {}found {count} {noun}{}",
                palette.error, palette.reset
            );
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
        warnings: Vec::new(),
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
        warnings: Vec::new(),
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
