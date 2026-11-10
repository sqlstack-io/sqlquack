//! `convert_report`: what `sqlquack convert` reports for some SQL, for the command line and Python.

use sqlquack_convert::snowflake::catalogue::main::builtin_catalogue::builtin_catalogue;

use crate::cli::_helpers::convert_command::{analysis_report, chosen_options, refused_session};
use crate::cli::_helpers::project::project_choices;
use crate::cli::models::{ConvertArguments, ConvertReport};
use sqlquack_convert::snowflake::analysis::main::analyze::analyze;
use sqlquack_convert::snowflake::session::main::parse_session::parse_session;
use sqlquack_convert::snowflake::session::main::session_conversion_options::session_conversion_options;

/// The report `convert` prints for `sql` under `arguments`, or why it cannot be made (exit 2).
pub fn convert_report(arguments: &ConvertArguments, sql: &str) -> Result<ConvertReport, String> {
    let project = project_choices(
        &arguments.conversion,
        (
            arguments.schema.as_ref(),
            &arguments.session,
            arguments.overrides.as_deref(),
        ),
    )?;
    let (schema, warnings, choices) = (project.schema, project.warnings, project.choices);
    let arguments = &ConvertArguments {
        session: project.session,
        overrides: project.overrides,
        verified_overrides: project
            .verified_overrides
            .into_iter()
            .chain(arguments.verified_overrides.iter().cloned())
            .collect(),
        conversion: project.conversion,
        ..arguments.clone()
    };
    let parameters = parse_session(&arguments.session)?;
    let (options, profile) = match session_conversion_options(&choices, &parameters) {
        Ok(matched) => matched,
        Err(reason) => {
            return Ok(ConvertReport {
                warnings,
                ..refused_session(reason)
            });
        }
    };
    let options = chosen_options(arguments, options)?;
    let catalogue = builtin_catalogue();
    let analysis = analyze(sql, &catalogue, (&schema, &options));
    if analysis.parsed && analysis.statements == 0 {
        return Err("the input holds no SQL statement".to_owned());
    }
    let report = analysis_report(
        &analysis,
        (sql, arguments.compact),
        (&schema, &options),
        (&choices, &parameters, profile),
    )?;
    Ok(ConvertReport { warnings, ..report })
}
