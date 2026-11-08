//! `convert_report`: what `sqlquack convert` reports for some SQL, for the command line and Python.

use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;

use crate::cli::_helpers::convert_command::{analysis_report, chosen_options, refused_session};
use crate::cli::_helpers::project::project_choices;
use crate::cli::models::{ConvertArguments, ConvertReport};
use sqlquack_convert::analysis::main::analyze::analyze;
use sqlquack_rules::session::main::parse_session::parse_session;
use sqlquack_rules::session::main::session_conversion_options::session_conversion_options;

/// The report `convert` prints for `sql` under `arguments`, or why it cannot be made (exit 2).
pub fn convert_report(arguments: &ConvertArguments, sql: &str) -> Result<ConvertReport, String> {
    let (policy, project) = project_choices(
        &arguments.conversion,
        (
            arguments.schema.as_deref(),
            &arguments.session,
            arguments.overrides.as_deref(),
        ),
    )?;
    let schema = project.schema;
    let warnings = project.warnings;
    let arguments = &ConvertArguments {
        session: project.session,
        overrides: project.overrides,
        conversion: project.conversion,
        ..arguments.clone()
    };
    let parameters = parse_session(&arguments.session)?;
    let session = (
        arguments.conversion.strict,
        arguments.conversion.utc_session,
    );
    let (options, profile) = match session_conversion_options(policy.as_ref(), session, &parameters)
    {
        Ok(matched) => matched,
        Err(reason) => {
            return Ok(ConvertReport {
                warnings,
                ..refused_session(reason)
            });
        }
    };
    let options = chosen_options(arguments, policy.as_ref(), options)?;
    let catalogue = builtin_catalogue();
    let analysis = analyze(
        sql,
        SourceDialect::Snowflake,
        &catalogue,
        (&schema, &options),
    );
    if analysis.parsed && analysis.statements == 0 {
        return Err("the input holds no SQL statement".to_owned());
    }
    let report = analysis_report(
        &analysis,
        (sql, arguments.compact),
        (&schema, &options, policy.as_ref()),
        (&arguments.conversion, &parameters, profile),
    )?;
    Ok(ConvertReport { warnings, ..report })
}
