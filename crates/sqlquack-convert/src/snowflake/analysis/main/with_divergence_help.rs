//! `with_divergence_help`: SQ0008 diagnostics worded from their known difference, with the fix that works.

use std::collections::BTreeMap;

use sqlquack_diagnostics::diagnostics::models::Diagnostic;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_policy::known_differences::main::known_difference::known_difference;
use sqlquack_policy::policy::constants::{
    PROJECT_UDF, SESSION_TIME_ZONE, SESSION_TIMEZONE_UTC, TIMEZONE_PARAMETER,
};

use crate::snowflake::analysis::constants::{NEW_CONFIG_FILE, STRICT_REFUSES};
use crate::snowflake::analysis::main::assumption_id::assumption_id;
use crate::snowflake::analysis::models::ConversionChoices;

/// `diagnostics`, each SQ0008 one worded from its known difference, with the fix that works here.
pub fn with_divergence_help(
    refusals: &[Refusal],
    diagnostics: Vec<Diagnostic>,
    (choices, parameters): (&ConversionChoices, &BTreeMap<String, String>),
) -> Vec<Diagnostic> {
    refusals
        .iter()
        .zip(diagnostics)
        .map(|(refusal, mut diagnostic)| {
            if let Some(id) = refusal.divergence() {
                let id = assumption_id(id);
                if let Some(entry) = known_difference(id) {
                    diagnostic.message = entry.title.to_owned();
                    diagnostic.label = entry.summary().to_owned();
                    diagnostic.notes = entry.notes();
                }
                diagnostic.help = Some(divergence_help(id, (choices, parameters)));
            }
            diagnostic
        })
        .collect()
}

/// The one fix for known difference `id` that works under the caller's choices.
fn divergence_help(
    id: &str,
    (choices, parameters): (&ConversionChoices, &BTreeMap<String, String>),
) -> String {
    let zone = parameters
        .get(TIMEZONE_PARAMETER)
        .filter(|zone| *zone != SESSION_TIME_ZONE);
    if let (SESSION_TIMEZONE_UTC, Some(zone)) = (id, zone) {
        return format!(
            "this matches Snowflake only in a UTC session, and the session time zone is {zone}; \
             run both the Snowflake and the DuckDB session in UTC, and convert for that session"
        );
    }
    if choices.strict {
        return STRICT_REFUSES.to_owned();
    }
    let file = choices.config_file.as_deref().unwrap_or(NEW_CONFIG_FILE);
    match id {
        PROJECT_UDF => format!(
            "set python_runtime = true under [project] in {file} if a Python runtime registers \
             the project's functions"
        ),
        _ => format!(
            "allow it with {id} = true under [allow] in {file}; `sqlquack explain {id}` says \
             when it is safe"
        ),
    }
}
