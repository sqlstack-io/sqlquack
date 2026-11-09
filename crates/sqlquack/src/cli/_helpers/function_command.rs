//! `sqlquack function`: a SQL project function's body as a DuckDB macro, or why it is refused.

use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use serde::Serialize;
use sqlquack_blocks::macros::main::helper_statements::helper_statements;
use sqlquack_diagnostics::classification::models::SourceDialect;
use sqlquack_diagnostics::diagnostics::constants::SESSION_PARAMETERS;
use sqlquack_diagnostics::diagnostics::main::diagnose::diagnose;
use sqlquack_diagnostics::diagnostics::main::render_diagnostics::render_diagnostics;
use sqlquack_diagnostics::diagnostics::main::unplaced_diagnostic::unplaced_diagnostic;
use sqlquack_diagnostics::diagnostics::models::Diagnostic;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_rules::conversion::models::ConvertedFunction;

use crate::census::main::resolve_references::resolve_references;
use crate::cli::_helpers::evidence::caller_choices;
use crate::cli::_helpers::evidence::with_divergence_help;
use crate::cli::_helpers::project::project_choices;
use crate::cli::models::{FunctionArguments, FunctionInputs};
use sqlquack_convert::analysis::main::analyze_function::analyze_function;
use sqlquack_rules::session::main::parse_session::parse_session;
use sqlquack_rules::session::main::session_conversion_options::session_conversion_options;

/// The path naming standard input.
const STDIN_PATH: &str = "-";
/// How diagnostics name standard input.
const STDIN_NAME: &str = "<stdin>";

/// Why a function did not convert.
enum Failure {
    /// The command could not run: an argument or file is unusable.
    Usage(String),
    /// The body was read and refused, for these reasons.
    Refused {
        body: String,
        diagnostics: Vec<Diagnostic>,
    },
}

/// What a converted `function` prints: the function and the helper macros its body calls.
#[derive(Serialize)]
struct FunctionOutput<'converted> {
    #[serde(flatten)]
    converted: &'converted ConvertedFunction,
    macros: Vec<String>,
}

/// What `function` prints on standard output when the body is refused.
#[derive(Serialize)]
struct FunctionRefusal<'report> {
    converted: bool,
    name: &'report str,
    refusals: &'report [Diagnostic],
}

pub(crate) fn function_command(
    arguments: &FunctionArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let _ = writeln!(stderr, "sqlquack function: converting {}…", arguments.name);
    let converted = function_inputs(arguments)
        .map_err(Failure::Usage)
        .and_then(|inputs| convert(arguments, &inputs));
    match converted {
        Ok(converted) => {
            let _ = writeln!(
                stdout,
                "{}",
                serde_json::to_string_pretty(&FunctionOutput {
                    converted: &converted,
                    macros: helper_statements(&[&converted.body]),
                })
                .unwrap_or_default()
            );
            let _ = writeln!(
                stderr,
                "sqlquack function: done, {} converts",
                arguments.name
            );
            ExitCode::SUCCESS
        }
        Err(Failure::Usage(error)) => {
            let _ = writeln!(stderr, "sqlquack function: failed: {error}");
            ExitCode::from(2)
        }
        Err(Failure::Refused { body, diagnostics }) => {
            let report = FunctionRefusal {
                converted: false,
                name: &arguments.name,
                refusals: &diagnostics,
            };
            let _ = writeln!(
                stdout,
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_default()
            );
            let _ = write!(
                stderr,
                "{}",
                render_diagnostics(&diagnostics, &body, &shown_path(&arguments.body))
            );
            let _ = writeln!(stderr, "sqlquack function: refused {}", arguments.name);
            ExitCode::FAILURE
        }
    }
}

/// The policy, session parameters, schema and choices of the project and flags; an error is a usage error.
fn function_inputs(arguments: &FunctionArguments) -> Result<FunctionInputs, String> {
    let (policy, project) =
        project_choices(&arguments.conversion, (None, &arguments.session, None))?;
    Ok(FunctionInputs {
        policy,
        parameters: parse_session(&project.session)?,
        schema: project.schema,
        conversion: project.conversion,
    })
}

/// How diagnostics name the body's file.
fn shown_path(path: &Path) -> String {
    match path.as_os_str() == STDIN_PATH {
        true => STDIN_NAME.to_owned(),
        false => path.display().to_string(),
    }
}

/// The body, from its file or from standard input when the path is `-`.
fn read_body(path: &Path) -> Result<String, String> {
    match path.as_os_str() == STDIN_PATH {
        true => {
            let mut body = String::new();
            std::io::stdin()
                .read_to_string(&mut body)
                .map_err(|error| format!("cannot read standard input: {error}"))?;
            Ok(body)
        }
        false => std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display())),
    }
}

fn convert(
    arguments: &FunctionArguments,
    FunctionInputs {
        policy,
        parameters,
        schema,
        conversion,
    }: &FunctionInputs,
) -> Result<ConvertedFunction, Failure> {
    let spec = schema.functions.get(&arguments.name).ok_or_else(|| {
        Failure::Usage(format!(
            "no function {} is declared; declare it in the project's functions folder",
            arguments.name
        ))
    })?;
    let body = resolve_references(&read_body(&arguments.body).map_err(Failure::Usage)?);
    let options = match session_conversion_options(
        policy.as_ref(),
        (conversion.strict, conversion.utc_session),
        parameters,
    ) {
        Ok((options, _)) => options,
        Err(reason) => {
            return Err(Failure::Refused {
                diagnostics: unplaced_diagnostic(SESSION_PARAMETERS, reason)
                    .into_iter()
                    .collect(),
                body,
            });
        }
    };
    let options = match policy {
        Some(policy) => caller_choices(options, policy, conversion),
        None => options,
    };
    analyze_function(
        (&body, spec),
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (schema, &options),
    )
    .map_err(|refusals| Failure::Refused {
        diagnostics: with_divergence_help(
            &refusals,
            diagnose(&refusals, &body),
            (policy.as_ref(), conversion, parameters),
        ),
        body,
    })
}
