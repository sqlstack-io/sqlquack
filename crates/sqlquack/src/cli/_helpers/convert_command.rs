//! `sqlquack convert`, `check`, `explain` and `run`: DuckDB SQL for Snowflake SQL, its problems, its codes, and its rows.

use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use sqlquack_blocks::layout::main::layout_sql::layout_sql;
use sqlquack_blocks::macros::main::explain_helper::explain_helper;
use sqlquack_convert::snowflake::conversion::models::ConversionOptions;
use sqlquack_diagnostics::diagnostics::constants::{CODES, SESSION_PARAMETERS};
use sqlquack_diagnostics::diagnostics::main::diagnose::diagnose;
use sqlquack_diagnostics::diagnostics::main::explain_code::explain_code;
use sqlquack_diagnostics::diagnostics::main::render_styled_diagnostics::render_styled_diagnostics;
use sqlquack_diagnostics::diagnostics::main::unplaced_diagnostic::unplaced_diagnostic;
use sqlquack_diagnostics::diagnostics::main::warning_at::warning_at;
use sqlquack_diagnostics::diagnostics::models::{Diagnostic, Palette};
use sqlquack_diagnostics::provenance::main::merge_provenance::merge_provenance;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_format::FormatDialect;
use sqlquack_policy::config::constants::CONFIG_FILE_NAME;
use sqlquack_policy::config::main::load_config::load_config;
use sqlquack_policy::config::models::UserConfig;
use sqlquack_policy::known_differences::constants::KNOWN_DIFFERENCES;
use sqlquack_policy::known_differences::main::known_difference::known_difference;
use sqlquack_policy::policy::constants::{PROJECT_UDF, SESSION_TIME_ZONE, TIMEZONE_PARAMETER};
use sqlquack_policy::policy::main::evidence_policy::evidence_policy;
use sqlquack_policy::policy::models::DivergenceFailure;
use sqlquack_semantics::snowflake::typing::types::Schema;
use std::collections::{BTreeMap, BTreeSet};

use crate::analysis::main::check_source::check_source;
use crate::analysis::main::override_macros::override_macros;
use crate::analysis::main::python_functions::python_functions;
use crate::analysis::main::refused_construct::refused_construct;
use crate::analysis::main::with_helpers::with_helpers;
use crate::analysis::main::with_overrides::with_overrides;
use crate::cli::_helpers::project::project_choices;
use crate::cli::models::{
    CheckArguments, CheckReport, ColorArguments, ConvertArguments, ConvertRefusal, ConvertReport,
    ExplainArguments, KnownDifferenceUse, OutputArguments, OutputFormat, OverrideListing,
    RunArguments, RunPrinted,
};
use crate::run::main::run_query::run_query;
use crate::run::models::RunInputs;
use crate::terminal::main::highlight_sql::highlight_sql;
use sqlquack_convert::snowflake::analysis::main::assumption_id::assumption_id;
use sqlquack_convert::snowflake::analysis::models::{Analysis, ConversionChoices};
use sqlquack_convert::snowflake::session::main::configured_options::configured_options;

use crate::cli::main::convert_report::convert_report;
use sqlquack_convert::snowflake::analysis::main::with_divergence_help::with_divergence_help;

/// What allows a known difference that the configuration does not allow, in `convert --json`.
const ALLOWED_BY_DEFAULT: &str = "default";
/// What allows `project_udf` when a Python runtime registers the project's functions.
const ALLOWED_BY_RUNTIME: &str = "python_runtime";
/// How `explain --differences` marks a known difference that can return a different value.
const VALUE_MARK: &str = "value";
/// How `explain --differences` marks one that raises an error wherever the results could differ.
const ERROR_MARK: &str = "error";
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
        ("convert", arguments.verbose),
        &report,
        (&sql, arguments.path.as_deref()),
        palette,
    );
    let _ = stderr.write_all(findings.as_bytes());
    match report.converted {
        true => {
            let _ = writeln!(
                stderr,
                "sqlquack convert: {}done, converted{}{}",
                palette.success,
                palette.reset,
                allowed_summary(&report)
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
        verbose: false,
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
    let findings = conversion_findings(
        ("run", false),
        &report,
        (&sql, arguments.path.as_deref()),
        palette,
    );
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
    .and_then(|project| {
        run_query(&RunInputs {
            source: &sql,
            converted: &report.sql,
            macros: &report.macros,
            profile: &report.profile,
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
    let allowed = allowed_summary(&report);
    let (exit, verdict) = match outcome.differences {
        None => (
            ExitCode::SUCCESS,
            format!("sqlquack run: done, {count}{allowed}\n"),
        ),
        Some(differences) if differences.is_empty() => (
            ExitCode::SUCCESS,
            format!("sqlquack run: done, {count} as expected{allowed}\n"),
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

/// What `command` noticed converting `sql`, each known difference used when `verbose`, and refusals.
fn conversion_findings(
    (command, verbose): (&str, bool),
    report: &ConvertReport,
    (sql, path): (&str, Option<&Path>),
    palette: &Palette,
) -> String {
    let mut text = String::new();
    for warning in &report.warnings {
        text.push_str(&format!("sqlquack {command}: warning: {warning}\n"));
    }
    for note in &report.session_notes {
        text.push_str(&format!("sqlquack {command}: note: {note}\n"));
    }
    if verbose && !report.known_differences.is_empty() {
        let warnings: Vec<Diagnostic> = report
            .known_differences
            .iter()
            .map(|used| difference_warning(used, sql))
            .collect();
        text.push_str(&render_styled_diagnostics(
            &warnings,
            sql,
            &source_name(path),
            palette,
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
        let noun = match report.refusals.len() {
            1 => "refusal",
            _ => "refusals",
        };
        text.push_str(&format!(
            "sqlquack {command}: {}refused, {} {noun}{}\n",
            palette.error,
            report.refusals.len(),
            palette.reset
        ));
    }
    text
}

/// The warning `--verbose` prints for one use of a known difference.
fn difference_warning(used: &KnownDifferenceUse, sql: &str) -> Diagnostic {
    let label = known_difference(&used.name).map_or("", |entry| entry.summary());
    let allowed = match (used.allowed_by.as_str(), &used.reason) {
        (ALLOWED_BY_DEFAULT, _) | (_, None) => format!("allowed by {}", used.allowed_by),
        (file, Some(reason)) => format!("allowed by {file}: {reason}"),
    };
    warning_at(
        sql,
        used.location,
        (used.title.clone(), label.to_owned(), vec![allowed]),
    )
}

/// The known differences a converted report allows, as the end of its summary line.
fn allowed_summary(report: &ConvertReport) -> String {
    let names: BTreeSet<&str> = report
        .known_differences
        .iter()
        .map(|used| used.name.as_str())
        .collect();
    let listed = names.iter().copied().collect::<Vec<&str>>().join(", ");
    match names.len() {
        0 => String::new(),
        1 => format!(" (1 known difference allowed: {listed} — sqlquack explain {listed})"),
        count => {
            format!(" ({count} known differences allowed: {listed} — sqlquack explain <name>)")
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
    let explained = match (arguments.differences, arguments.codes, &arguments.code) {
        (true, _, _) => known_differences(Path::new(CONFIG_FILE_NAME)),
        (false, true, _) => Ok(codes()),
        (false, false, Some(key)) => explain_key(key).ok_or_else(|| {
            format!(
                "no diagnostic code, name, former name, known difference or helper macro is `{key}`"
            )
        }),
        (false, false, None) => Err("name a code, a known difference or a helper macro".into()),
    };
    match explained {
        Ok(text) => {
            let _ = write!(stdout, "{text}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack explain: {error}");
            ExitCode::from(2)
        }
    }
}

fn explain_key(key: &str) -> Option<String> {
    explain_code(key)
        .or_else(|| explain_difference(key))
        .or_else(|| explain_helper(key))
}

/// A known difference's title, what differs, its details, when it is fine or wrong, and how it fails.
fn explain_difference(name: &str) -> Option<String> {
    let entry = known_difference(name)?;
    let details = entry
        .details
        .map_or(String::new(), |details| format!("{details}\n\n"));
    Some(format!(
        "{name}: {title}\n\n{differs}\n\n{details}fine when: {fine}\nwrong when: {wrong}\nfails with: {fails}\n",
        name = entry.name,
        title = entry.title,
        differs = entry.differs,
        fine = entry.fine_when,
        wrong = entry.wrong_when,
        fails = entry.fails_with(),
    ))
}

/// Every known difference, marked allowed by the configuration at `config_path`, by default, or refused.
fn known_differences(config_path: &Path) -> Result<String, String> {
    let config_file = Some(config_path).filter(|path| path.is_file());
    let config = match config_file {
        Some(path) => load_config(path, evidence_policy()).map_err(|error| error.to_string())?,
        None => UserConfig::default(),
    };
    let choices = ConversionChoices {
        config_file: config_file.map(|path| path.display().to_string()),
        allow: config.allow,
        python_runtime: config.project.python_runtime,
        ..ConversionChoices::default()
    };
    let options = configured_options(&choices);
    let rules = match config_file {
        Some(_) => format!("as the defaults and ./{CONFIG_FILE_NAME} treat them"),
        None => format!("as the defaults treat them, with no ./{CONFIG_FILE_NAME}"),
    };
    let statuses: Vec<String> = KNOWN_DIFFERENCES
        .iter()
        .map(|entry| {
            match (
                options.divergences.contains_key(entry.name),
                allowed_by(&choices, entry.name),
            ) {
                (false, _) => "refused".to_owned(),
                (true, by) if by != ALLOWED_BY_DEFAULT => format!("allowed by {by}"),
                (true, _) => "allowed by default".to_owned(),
            }
        })
        .collect();
    let name_width = KNOWN_DIFFERENCES
        .iter()
        .map(|entry| entry.name.len())
        .max()
        .unwrap_or(0);
    let status_width = statuses.iter().map(String::len).max().unwrap_or(0);
    let lines: String = KNOWN_DIFFERENCES
        .iter()
        .zip(&statuses)
        .map(|(entry, status)| {
            let mark = match evidence_policy()
                .divergences
                .get(entry.name)
                .is_some_and(|divergence| divergence.failure == DivergenceFailure::Value)
            {
                true => VALUE_MARK,
                false => ERROR_MARK,
            };
            format!(
                "  {name:<name_width$}  {status:<status_width$}  {mark:<5}  {title}\n",
                name = entry.name,
                title = entry.title,
            )
        })
        .collect();
    Ok(format!(
        "Known differences between Snowflake and the converted SQL, {rules}:\n\n{lines}\n\
         `{VALUE_MARK}` marks a difference that can return a different value with no error, and \
         `{ERROR_MARK}` one that raises an error instead.\n\
         `sqlquack explain <name>` describes one.\n"
    ))
}

/// Every diagnostic code with its name and title.
fn codes() -> String {
    let name_width = CODES
        .iter()
        .map(|entry| entry.name.len())
        .max()
        .unwrap_or(0);
    let lines: String = CODES
        .iter()
        .map(|entry| {
            format!(
                "  {code}  {name:<name_width$}  {title}\n",
                code = entry.code,
                name = entry.name,
                title = entry.title,
            )
        })
        .collect();
    format!("Diagnostic codes:\n\n{lines}\n`sqlquack explain <code>` describes one.\n")
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
        known_differences: Vec::new(),
        notes: Vec::new(),
        session_notes: Vec::new(),
        warnings: Vec::new(),
        python_functions: Vec::new(),
        macros: Vec::new(),
        overrides: Vec::new(),
    }
}

/// The session's options with the user overrides applied.
pub(crate) fn chosen_options(
    arguments: &ConvertArguments,
    options: ConversionOptions,
) -> Result<ConversionOptions, String> {
    with_overrides(
        options,
        (
            arguments.overrides.as_deref(),
            &arguments.verified_overrides,
        ),
    )
}

/// The report of a finished analysis of `sql`, its SQL and helper macros laid out over lines unless `compact`.
pub(crate) fn analysis_report(
    analysis: &Analysis,
    (sql, compact): (&str, bool),
    (schema, options): (&Schema, &ConversionOptions),
    (choices, parameters, profile): (&ConversionChoices, &BTreeMap<String, String>, String),
) -> Result<ConvertReport, String> {
    let evidence = evidence_policy();
    let converted = analysis.parsed && analysis.refusals.is_empty();
    let provenance = merge_provenance(analysis.conversions.iter().map(|done| &done.provenance));
    let assumptions: Vec<String> = provenance
        .divergences
        .keys()
        .filter(|id| {
            evidence
                .divergences
                .get(assumption_id(id))
                .is_some_and(|divergence| divergence.assumption)
        })
        .cloned()
        .collect();
    let divergence_failures = divergence_failures(&provenance.divergences);
    let session_notes: Vec<String> = parameters
        .get(TIMEZONE_PARAMETER)
        .filter(|zone| *zone != SESSION_TIME_ZONE)
        .map(|zone| {
            format!(
                "the session time zone is {zone}, not UTC, so statements that depend on it are \
                 refused"
            )
        })
        .into_iter()
        .collect();
    let notes: Vec<String> = session_notes
        .iter()
        .cloned()
        .chain(
            provenance
                .divergences
                .keys()
                .filter(|id| allowed_by(choices, assumption_id(id)) == ALLOWED_BY_DEFAULT)
                .filter_map(|id| options.divergences.get(id.as_str()))
                .map(|divergence| divergence.reason.clone()),
        )
        .collect();
    let converted_sql = match converted {
        true => analysis
            .conversions
            .iter()
            .map(|done| match compact {
                true => done.sql.clone(),
                false => layout_sql(&done.sql, FormatDialect::DuckDB),
            })
            .collect::<Vec<String>>()
            .join(";\n"),
        false => String::new(),
    };
    let macros: Vec<String> = with_helpers(
        override_macros(&provenance.divergences, options),
        &converted_sql,
    )
    .into_iter()
    .map(|statement| match compact {
        true => statement,
        false => layout_sql(&statement, FormatDialect::DuckDB),
    })
    .collect();
    Ok(ConvertReport {
        converted,
        profile,
        known_differences: known_difference_uses(analysis, choices),
        notes,
        session_notes,
        warnings: Vec::new(),
        sql: converted_sql,
        refusals: analysis
            .refusals
            .iter()
            .zip(with_divergence_help(
                &analysis.refusals,
                diagnose(&analysis.refusals, sql),
                (choices, parameters),
            ))
            .map(convert_refusal)
            .collect(),
        python_functions: python_functions(&provenance.divergences, schema)?,
        macros,
        overrides: override_listings(options),
        divergences: listed(provenance.divergences),
        assumptions,
        divergence_failures,
    })
}

/// Every use of a known difference in the converted statements, each once, with who allowed it and why.
fn known_difference_uses(
    analysis: &Analysis,
    choices: &ConversionChoices,
) -> Vec<KnownDifferenceUse> {
    let mut uses: Vec<KnownDifferenceUse> = Vec::new();
    for used in analysis
        .conversions
        .iter()
        .flat_map(|done| &done.divergence_uses)
    {
        let name = assumption_id(&used.id);
        let entry = known_difference(name);
        let allowed_by = allowed_by(choices, name);
        let listed = KnownDifferenceUse {
            name: name.to_owned(),
            title: entry.map_or_else(|| name.to_owned(), |entry| entry.title.to_owned()),
            fails_with: entry.map_or("", |entry| entry.fails_with()).to_owned(),
            reason: evidence_policy()
                .divergences
                .get(name)
                .filter(|_| allowed_by != ALLOWED_BY_DEFAULT)
                .map(|divergence| divergence.reason.clone()),
            allowed_by,
            construct: used.construct.clone(),
            line: used.location.map(|location| location.line),
            column: used.location.map(|location| location.column),
            location: used.location,
        };
        if !uses.iter().any(|seen| {
            (&seen.name, &seen.construct, seen.location)
                == (&listed.name, &listed.construct, listed.location)
        }) {
            uses.push(listed);
        }
    }
    uses
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

/// What allows the known difference `name`: the configuration file, the Python runtime, or the defaults.
fn allowed_by(choices: &ConversionChoices, name: &str) -> String {
    match (&choices.config_file, choices.allow.get(name)) {
        (Some(file), Some(true)) => file.clone(),
        _ if name == PROJECT_UDF && choices.python_runtime => ALLOWED_BY_RUNTIME.to_owned(),
        _ => ALLOWED_BY_DEFAULT.to_owned(),
    }
}

/// Each relied-on divergence's failure, as the evidence declares it.
fn divergence_failures(
    divergences: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeMap<String, DivergenceFailure> {
    let policy = evidence_policy();
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
