//! `session_conversion_options`: conversion options for a caller's Snowflake session parameters.

use std::collections::BTreeMap;

use crate::snowflake::analysis::models::ConversionChoices;
use crate::snowflake::conversion::models::ConversionOptions;
use sqlquack_policy::policy::constants::{
    DATABASE_PARAMETER, DEFAULT_SESSION_PROFILE, QUOTED_IDENTIFIERS_IGNORE_CASE, SCHEMA_PARAMETER,
    SESSION_TIME_ZONE, SESSION_TIMEZONE_UTC, TIMEZONE_PARAMETER,
};
use sqlquack_policy::policy::main::evidence_policy::evidence_policy;
use sqlquack_policy::policy::models::SessionValue;

use crate::snowflake::session::_helpers::session_profiles::matching_profile;
use crate::snowflake::session::main::configured_options::configured_options;

/// Options for a session with `parameters`, under the recorded profile they match, and its name.
pub fn session_conversion_options(
    choices: &ConversionChoices,
    parameters: &BTreeMap<String, String>,
) -> Result<(ConversionOptions, String), String> {
    let evidence = evidence_policy();
    let profile = matching_profile(
        evidence,
        parameters,
        (DEFAULT_SESSION_PROFILE, TIMEZONE_PARAMETER),
    )?;
    let ignore_case = evidence
        .session_profiles
        .get(&profile)
        .and_then(|matched| matched.snowflake.get(QUOTED_IDENTIFIERS_IGNORE_CASE))
        .is_some_and(|value| value.text() == SessionValue::Boolean(true).text());
    let mut options = ConversionOptions {
        quoted_identifiers_ignore_case: ignore_case,
        ..configured_options(choices)
    };
    let zone = parameters.get(TIMEZONE_PARAMETER);
    if zone.is_some_and(|zone| zone != SESSION_TIME_ZONE) {
        options.divergences.remove(SESSION_TIMEZONE_UTC);
    }
    Ok((with_namespace(options, parameters), profile))
}

/// `options` with the session's DATABASE and SCHEMA, upper case unless written in double quotes.
fn with_namespace(
    options: ConversionOptions,
    parameters: &BTreeMap<String, String>,
) -> ConversionOptions {
    let resolved = |name: &str| {
        parameters.get(name).map(|value| {
            match value.len() > 1 && value.starts_with('"') && value.ends_with('"') {
                true => value[1..value.len() - 1].replace("\"\"", "\""),
                false => value.to_ascii_uppercase(),
            }
        })
    };
    ConversionOptions {
        current_database: resolved(DATABASE_PARAMETER),
        current_schema: resolved(SCHEMA_PARAMETER),
        ..options
    }
}
