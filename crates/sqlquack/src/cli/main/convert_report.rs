//! `convert_report`: what `sqlquack convert` reports for some SQL, for the command line and Python.

use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;

use crate::analysis::main::analyze::analyze;
use crate::cli::_helpers::convert_command::{analysis_report, chosen_options, refused_session};
use crate::cli::_helpers::evidence::chosen_policy;
use crate::cli::_helpers::schema::load_schema;
use crate::cli::models::{ConvertArguments, ConvertReport};
use sqlquack_rules::session::main::parse_session::parse_session;
use sqlquack_rules::session::main::session_conversion_options::session_conversion_options;

/// The report `convert` prints for `sql` under `arguments`, or why it cannot be made (exit 2).
pub fn convert_report(arguments: &ConvertArguments, sql: &str) -> Result<ConvertReport, String> {
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
    let session = (
        arguments.conversion.strict,
        arguments.conversion.utc_session,
    );
    let (options, profile) = match session_conversion_options(policy.as_ref(), session, &parameters)
    {
        Ok(matched) => matched,
        Err(reason) => return Ok(refused_session(reason)),
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
    analysis_report(
        &analysis,
        sql,
        (&schema, &options, policy.as_ref()),
        (&parameters, profile),
    )
}
