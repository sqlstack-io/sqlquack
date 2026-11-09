//! Whether a result was recorded under the session parameters its case's profile sets and pins.

use sqlquack_policy::policy::models::SessionValue;

use crate::cases::models::LoadedCase;
use crate::results::models::{ResultMetadata, SessionDefaults};

/// How a result's recorded session compares with its profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SessionCheck {
    /// Every set and pinned parameter had the profile's value.
    Matches,
    /// Recorded before pinning, and no account defaults are recorded to vouch for it.
    Unproven,
    /// A parameter had another value.
    Mismatch(String),
}

/// Effective values when the result has them; the recorded account defaults otherwise.
pub(crate) fn session_check(
    case: &LoadedCase,
    metadata: &ResultMetadata,
    defaults: Option<&SessionDefaults>,
) -> SessionCheck {
    if let Some(effective) = &metadata.effective_parameters {
        let expected = case
            .session_parameters
            .iter()
            .chain(&case.pinned_parameters);
        for (key, value) in expected {
            let actual = effective.get(key).cloned().flatten();
            if !actual.as_deref().is_some_and(|actual| same(actual, value)) {
                return SessionCheck::Mismatch(format!(
                    "session parameter {key} was recorded as {}, but the profile sets {}",
                    actual.as_deref().unwrap_or("nothing"),
                    value.text()
                ));
            }
        }
        return SessionCheck::Matches;
    }
    if case.pinned_parameters.is_empty() {
        return SessionCheck::Matches;
    }
    let Some(defaults) = defaults else {
        return SessionCheck::Unproven;
    };
    for (key, value) in &case.pinned_parameters {
        let default = defaults.parameters.get(key);
        if !default.is_some_and(|default| same(default, value)) {
            return SessionCheck::Mismatch(format!(
                "recorded before {key} was pinned, and the account default is {}, not {}",
                default.map_or("unrecorded", String::as_str),
                value.text()
            ));
        }
    }
    SessionCheck::Matches
}

/// Whether a reported value is the profile's, ignoring case and surrounding space.
fn same(actual: &str, value: &SessionValue) -> bool {
    actual.trim().eq_ignore_ascii_case(value.text().trim())
}
