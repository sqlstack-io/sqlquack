//! Reading a caller's session parameters and matching them with the recorded session profiles.

use std::collections::{BTreeMap, BTreeSet};

use sqlquack_policy::policy::constants::{
    DATABASE_PARAMETER, RESULT_NEUTRAL_SESSION_PARAMETERS, SCHEMA_PARAMETER,
};
use sqlquack_policy::policy::models::{Policy, SessionValue};

/// The first profile matching `given`, unset ones as `default` pins, `ignored` never compared.
pub(crate) fn matching_profile(
    policy: &Policy,
    given: &BTreeMap<String, String>,
    (default, ignored): (&str, &str),
) -> Result<String, String> {
    let defaults = policy
        .session_profiles
        .get(default)
        .ok_or_else(|| format!("the policy has no session profile `{default}`"))?;
    let neutral: BTreeSet<&str> = RESULT_NEUTRAL_SESSION_PARAMETERS
        .iter()
        .map(|(name, _)| *name)
        .collect();
    let mut known: BTreeSet<&str> = neutral.clone();
    known.extend([DATABASE_PARAMETER, SCHEMA_PARAMETER]);
    for profile in policy.session_profiles.values() {
        known.extend(
            profile
                .snowflake
                .keys()
                .chain(profile.pinned.keys())
                .map(String::as_str),
        );
    }
    let unknown: Vec<&str> = given
        .keys()
        .map(String::as_str)
        .filter(|name| !known.contains(name))
        .collect();
    if !unknown.is_empty() {
        return Err(format!(
            "unknown session parameters, which no recorded profile covers: {}",
            unknown.join(", ")
        ));
    }
    let effective = |name: &str| -> Option<String> {
        given.get(name).cloned().or_else(|| {
            defaults
                .snowflake
                .get(name)
                .or_else(|| defaults.pinned.get(name))
                .map(value_text)
        })
    };
    let mut unmatched: BTreeSet<String> = BTreeSet::new();
    for (name, profile) in &policy.session_profiles {
        let differing: Vec<&String> = profile
            .snowflake
            .iter()
            .chain(&profile.pinned)
            .filter(|(key, _)| key.as_str() != ignored && !neutral.contains(key.as_str()))
            .filter(|(key, value)| !same_value(effective(key).as_deref(), value))
            .map(|(key, _)| key)
            .collect();
        if differing.is_empty() {
            return Ok(name.clone());
        }
        unmatched.extend(
            differing
                .into_iter()
                .filter(|key| given.contains_key(*key))
                .cloned(),
        );
    }
    let described: Vec<String> = unmatched
        .iter()
        .map(|name| format!("{name} = {}", given.get(name).cloned().unwrap_or_default()))
        .collect();
    Err(format!(
        "the session parameters match no recorded session profile: {}",
        described.join(", ")
    ))
}

/// A parameter value as text: booleans in lower case, integers in digits, text as written.
pub(crate) fn value_text(value: &SessionValue) -> String {
    match value {
        SessionValue::Boolean(flag) => flag.to_string(),
        SessionValue::Integer(number) => number.to_string(),
        SessionValue::Text(text) => text.clone(),
    }
}

/// Whether a caller's value is the profile's: booleans ignoring case, others exactly.
fn same_value(given: Option<&str>, recorded: &SessionValue) -> bool {
    match (given, recorded) {
        (Some(given), SessionValue::Boolean(_)) => {
            given.eq_ignore_ascii_case(&value_text(recorded))
        }
        (Some(given), _) => given == value_text(recorded),
        (None, _) => false,
    }
}
