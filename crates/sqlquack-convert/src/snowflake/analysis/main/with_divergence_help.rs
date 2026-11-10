//! `with_divergence_help`: SQ0008 diagnostics worded from their known difference, with the fix that works.

use std::collections::BTreeMap;

use crate::snowflake::session::main::builtin_conversion_options::builtin_conversion_options;
use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::diagnostics::models::Diagnostic;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_policy::known_differences::main::known_difference::known_difference;
use sqlquack_policy::policy::constants::{
    CURRENT_TIME, DECLARED_OFFSET, PROJECT_UDF, SESSION_TIME_ZONE, SESSION_TIMEZONE_UTC,
    TIMEZONE_PARAMETER,
};
use sqlquack_policy::policy::models::{DivergenceFailure, DivergenceMode, Policy};

use crate::snowflake::analysis::main::assumption_id::assumption_id;
use crate::snowflake::analysis::models::ConversionChoices;

/// `diagnostics`, each SQ0008 one worded from its known difference, with the fix that works here.
pub fn with_divergence_help(
    refusals: &[Refusal],
    diagnostics: Vec<Diagnostic>,
    (policy, choices, parameters): (
        Option<&Policy>,
        &ConversionChoices,
        &BTreeMap<String, String>,
    ),
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
                    diagnostic.notes = vec![format!(
                        "fine when {}; wrong when {}",
                        sentence_part(entry.fine_when),
                        sentence_part(entry.wrong_when)
                    )];
                }
                let key = refusal.construct.as_ref().map(Construct::key);
                diagnostic.help = Some(divergence_help(
                    (id, key.as_deref()),
                    (policy, choices, parameters),
                ));
            }
            diagnostic
        })
        .collect()
}

/// `sentence` as the middle of another, without its full stop and with an ordinary word lower-cased.
fn sentence_part(sentence: &str) -> String {
    let sentence = sentence.trim_end_matches('.');
    let mut characters = sentence.chars();
    let (Some(first), Some(second)) = (characters.next(), characters.next()) else {
        return sentence.to_owned();
    };
    match first.is_ascii_uppercase() && (second.is_ascii_lowercase() || second == ' ') {
        true => first.to_ascii_lowercase().to_string() + &sentence[1..],
        false => sentence.to_owned(),
    }
}

/// The one fix for known difference `id` at construct `key` that works under the caller's choices.
fn divergence_help(
    (id, key): (&str, Option<&str>),
    (policy, choices, parameters): (
        Option<&Policy>,
        &ConversionChoices,
        &BTreeMap<String, String>,
    ),
) -> String {
    let explain = format!("`sqlquack explain {id}` says when it is safe");
    let Some(policy) = policy else {
        let defaults = builtin_conversion_options(false, false);
        let allowed_here = defaults.divergences.get(id).is_some_and(|allowed| {
            key.is_some_and(|key| allowed.constructs.iter().any(|c| c == key))
        });
        let clock = [CURRENT_TIME, SESSION_TIMEZONE_UTC].contains(&id);
        return match (
            allowed_here,
            choices.strict,
            choices.loud_divergences_only && clock,
        ) {
            (true, true, _) => "the defaults allow this outside strict mode".to_owned(),
            (true, false, true) => {
                "this can give a different value, so errors-only mode refuses it; the defaults \
                 allow it otherwise"
                    .to_owned()
            }
            (false, true, _) => format!(
                "allow it in strict mode in a sqlquack.toml with default = \"warn\" and strict = \
                 \"warn\" under [divergences.{id}], starting from a copy of the SQLQuack \
                 repository's; {explain}"
            ),
            _ => format!(
                "allow it in a sqlquack.toml with default = \"warn\" under [divergences.{id}], \
                 starting from a copy of the SQLQuack repository's; {explain}"
            ),
        };
    };
    let file = &choices.policy_file;
    let Some(divergence) = policy.divergences.get(id) else {
        return format!(
            "{file} has no [divergences.{id}]; copy it from the SQLQuack repository's sqlquack.toml"
        );
    };
    let zone = parameters
        .get(TIMEZONE_PARAMETER)
        .filter(|zone| *zone != SESSION_TIME_ZONE);
    if let (SESSION_TIMEZONE_UTC, Some(zone)) = (id, zone) {
        return format!(
            "this matches Snowflake only in a UTC session, and the session time zone is {zone}; \
             convert for a UTC session"
        );
    }
    if choices.loud_divergences_only && divergence.failure == DivergenceFailure::Value {
        return "this can give a different value, so errors-only mode refuses it".to_owned();
    }
    let confirmation = match id {
        SESSION_TIMEZONE_UTC => Some(("utc_session", choices.utc_session)),
        DECLARED_OFFSET => Some(("declared_offsets", choices.confirm_declared_offsets)),
        PROJECT_UDF => Some(("python_runtime", choices.python_runtime)),
        _ => None,
    };
    if let Some((key, false)) = confirmation {
        return format!("confirm it with `{key} = true` under [project.confirm] in {file}");
    }
    let mode = match choices.strict {
        true => divergence.strict,
        false => divergence.default,
    };
    let settings = match divergence.default {
        DivergenceMode::Refuse => "default = \"warn\" and strict = \"warn\"",
        DivergenceMode::Warn => "strict = \"warn\"",
    };
    match (mode, choices.strict) {
        (DivergenceMode::Refuse, true) => {
            format!("allow it in strict mode with {settings} under [divergences.{id}] in {file}")
        }
        (DivergenceMode::Refuse, false) => format!(
            "allow it with default = \"warn\" under [divergences.{id}] in {file}; {explain}"
        ),
        (DivergenceMode::Warn, _) => format!("allow it under [divergences.{id}] in {file}"),
    }
}
