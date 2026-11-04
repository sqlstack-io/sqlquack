//! `sqlquack function`: a SQL project function's body as a DuckDB macro, or why it is refused.

use std::collections::BTreeMap;
use std::io::Write;
use std::process::ExitCode;

use sqlquack_core::diagnostics::constants::SESSION_PARAMETERS;
use sqlquack_core::diagnostics::main::diagnose::diagnose;
use sqlquack_core::diagnostics::main::render_diagnostics::render_diagnostics;
use sqlquack_core::diagnostics::main::unplaced_diagnostic::unplaced_diagnostic;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_policy::policy::models::Policy;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_rules::conversion::models::ConvertedFunction;

use crate::analysis::main::analyze_function::analyze_function;
use crate::analysis::main::parse_session::parse_session;
use crate::analysis::main::session_conversion_options::session_conversion_options;
use crate::cli::_helpers::evidence::chosen_policy;
use crate::cli::_helpers::evidence::confirmed_offsets;
use crate::cli::_helpers::schema::load_schema;
use crate::cli::models::FunctionArguments;

pub(crate) fn function_command(
    arguments: &FunctionArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let _ = writeln!(stderr, "sqlquack function: converting {}…", arguments.name);
    let inputs = match function_inputs(arguments) {
        Ok(inputs) => inputs,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack function: failed: {error}");
            return ExitCode::from(2);
        }
    };
    match convert(arguments, &inputs) {
        Ok(converted) => {
            let _ = writeln!(
                stdout,
                "{}",
                serde_json::to_string_pretty(&converted).unwrap_or_default()
            );
            let _ = writeln!(
                stderr,
                "sqlquack function: done, {} converts",
                arguments.name
            );
            ExitCode::SUCCESS
        }
        Err(rendered) => {
            let _ = match rendered.ends_with('\n') {
                true => write!(stderr, "{rendered}"),
                false => writeln!(stderr, "sqlquack function: {rendered}"),
            };
            let _ = writeln!(stderr, "sqlquack function: refused {}", arguments.name);
            ExitCode::FAILURE
        }
    }
}

/// The policy and the caller's session parameters; an error is a usage error.
fn function_inputs(
    arguments: &FunctionArguments,
) -> Result<(Option<Policy>, BTreeMap<String, String>), String> {
    let policy = chosen_policy(&arguments.policy, arguments.builtin_policy)?;
    Ok((policy, parse_session(&arguments.session)?))
}

fn convert(
    arguments: &FunctionArguments,
    (policy, parameters): &(Option<Policy>, BTreeMap<String, String>),
) -> Result<ConvertedFunction, String> {
    let body_path = arguments.body.display().to_string();
    let (options, _) = session_conversion_options(
        policy.as_ref(),
        (arguments.strict, arguments.utc_session),
        parameters,
    )
    .map_err(|reason| {
        unplaced_diagnostic(SESSION_PARAMETERS, reason.clone()).map_or(reason, |diagnostic| {
            render_diagnostics(&[diagnostic], "", &body_path)
        })
    })?;
    let options = match policy {
        Some(policy) => confirmed_offsets(
            options,
            policy,
            (arguments.confirm_declared_offsets, arguments.python_runtime),
        ),
        None => options,
    };
    let schema = load_schema(&arguments.schema)?;
    let spec = schema
        .functions
        .get(&arguments.name)
        .ok_or_else(|| format!("the schema declares no function {}", arguments.name))?;
    let body = std::fs::read_to_string(&arguments.body)
        .map_err(|error| format!("cannot read {}: {error}", arguments.body.display()))?;
    analyze_function(
        (&body, spec),
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&schema, &options),
    )
    .map_err(|refusals| render_diagnostics(&diagnose(&refusals, &body), &body, &body_path))
}
