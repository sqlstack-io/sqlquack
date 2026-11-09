//! `convert_function`: a SQL project function's body converted as `sqlquack function` converts it.

use std::collections::BTreeMap;

use sqlquack_core::diagnostics::constants::SESSION_PARAMETERS;
use sqlquack_core::diagnostics::main::diagnose::diagnose;
use sqlquack_core::diagnostics::main::unplaced_diagnostic::unplaced_diagnostic;
use sqlquack_core::diagnostics::models::Diagnostic;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_policy::policy::models::Policy;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_rules::conversion::models::ConvertedFunction;
use sqlquack_rules::session::main::session_conversion_options::session_conversion_options;
use sqlquack_semantics::types::models::{FunctionSpec, Schema};

use crate::analysis::main::analyze_function::analyze_function;
use crate::analysis::main::caller_choices::caller_choices;
use crate::analysis::main::with_divergence_help::with_divergence_help;
use crate::analysis::models::ConversionChoices;

/// `body` as a DuckDB macro body under `policy` (built-in when `None`), or its diagnostics in order.
pub fn convert_function(
    (body, spec): (&str, &FunctionSpec),
    policy: Option<&Policy>,
    (choices, parameters): (&ConversionChoices, &BTreeMap<String, String>),
    schema: &Schema,
) -> Result<ConvertedFunction, Vec<Diagnostic>> {
    let options =
        match session_conversion_options(policy, (choices.strict, choices.utc_session), parameters)
        {
            Ok((options, _)) => options,
            Err(reason) => {
                return Err(unplaced_diagnostic(SESSION_PARAMETERS, reason)
                    .into_iter()
                    .collect());
            }
        };
    let options = match policy {
        Some(policy) => caller_choices(options, policy, choices),
        None => options,
    };
    analyze_function(
        (body, spec),
        SourceDialect::Snowflake,
        &builtin_catalogue(),
        (schema, &options),
    )
    .map_err(|refusals| {
        with_divergence_help(
            &refusals,
            diagnose(&refusals, body),
            (policy, choices, parameters),
        )
    })
}
