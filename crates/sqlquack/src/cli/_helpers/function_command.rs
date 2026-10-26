//! `sqlquack function`: a SQL project function's body as a DuckDB macro, or why it is refused.

use std::io::Write;
use std::process::ExitCode;

use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_rules::conversion::models::ConvertedFunction;

use crate::analysis::main::analyze_function::analyze_function;
use crate::cli::_helpers::evidence::divergence_options;
use crate::cli::_helpers::schema::load_schema;
use crate::cli::models::FunctionArguments;

pub(crate) fn function_command(
    arguments: &FunctionArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let _ = writeln!(stderr, "sqlquack function: converting {}…", arguments.name);
    match convert(arguments) {
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
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack function: refused: {error}");
            ExitCode::FAILURE
        }
    }
}

fn convert(arguments: &FunctionArguments) -> Result<ConvertedFunction, String> {
    let schema = load_schema(&arguments.schema)?;
    let spec = schema
        .functions
        .get(&arguments.name)
        .ok_or_else(|| format!("the schema declares no function {}", arguments.name))?;
    let body = std::fs::read_to_string(&arguments.body)
        .map_err(|error| format!("cannot read {}: {error}", arguments.body.display()))?;
    let (options, _) = divergence_options(
        Some(arguments.policy.as_path()).filter(|path| path.is_file()),
        arguments.strict,
    )?;
    analyze_function(
        (&body, spec),
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (&schema, &options),
    )
    .map_err(|refusals| {
        refusals
            .iter()
            .map(|refusal| refusal.reason.clone())
            .collect::<Vec<String>>()
            .join("; ")
    })
}
