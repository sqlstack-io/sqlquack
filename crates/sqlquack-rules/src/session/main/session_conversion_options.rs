//! `session_conversion_options`: conversion options for a caller's Snowflake session parameters.

use std::collections::BTreeMap;

use crate::conversion::models::ConversionOptions;
use sqlquack_policy::policy::constants::SESSION_TIMEZONE_UTC;
use sqlquack_policy::policy::constants::{
    DATABASE_PARAMETER, DEFAULT_SESSION_PROFILE, QUOTED_IDENTIFIERS_IGNORE_CASE,
    RESULT_NEUTRAL_SESSION_PARAMETERS, SCHEMA_PARAMETER, SESSION_TIME_ZONE, TIMEZONE_PARAMETER,
};
use sqlquack_policy::policy::models::{Policy, SessionValue};

use crate::session::_helpers::session_profiles::{matching_profile, value_text};
use crate::session::constants::BUILTIN_PROFILE;
use crate::session::main::confirm_utc_session::confirm_utc_session;
use crate::session::main::conversion_options::conversion_options;

/// Options for a session with `parameters`, under the recorded profile they match, and its name.
pub fn session_conversion_options(
    policy: Option<&Policy>,
    (strict, utc_session): (bool, bool),
    parameters: &BTreeMap<String, String>,
) -> Result<(ConversionOptions, String), String> {
    let Some(policy) = policy else {
        let unknown: Vec<&str> = parameters
            .keys()
            .map(String::as_str)
            .filter(|name| !builtin_accepts(name))
            .collect();
        return match unknown.is_empty() {
            true => Ok((
                with_namespace(ConversionOptions::default(), parameters),
                BUILTIN_PROFILE.to_owned(),
            )),
            false => Err(format!(
                "the built-in default records no session profiles, so it takes only TIMEZONE, \
                 DATABASE, SCHEMA and the result-neutral parameters, not {}; pass --policy with a policy file whose \
                 session profiles cover them, such as the repository's sqlquack.toml",
                unknown.join(", ")
            )),
        };
    };
    let profile = matching_profile(
        policy,
        parameters,
        (DEFAULT_SESSION_PROFILE, TIMEZONE_PARAMETER),
    )?;
    let ignore_case = policy
        .session_profiles
        .get(&profile)
        .and_then(|matched| matched.snowflake.get(QUOTED_IDENTIFIERS_IGNORE_CASE))
        .is_some_and(|value| value_text(value) == value_text(&SessionValue::Boolean(true)));
    let mut options = ConversionOptions {
        quoted_identifiers_ignore_case: ignore_case,
        ..conversion_options(policy, strict)
    };
    if utc_session {
        options = confirm_utc_session(options, policy);
    }
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

/// Whether the built-in default, which records no session profiles, takes parameter `name`.
fn builtin_accepts(name: &str) -> bool {
    [TIMEZONE_PARAMETER, DATABASE_PARAMETER, SCHEMA_PARAMETER].contains(&name)
        || RESULT_NEUTRAL_SESSION_PARAMETERS
            .iter()
            .any(|(neutral, _)| *neutral == name)
}
