//! Policy rules beyond the schema.

use std::collections::BTreeSet;

use crate::policy::_helpers::type_patterns::parse_type_pattern;
use crate::policy::constants::SCHEMA_VERSION;
use crate::policy::models::{
    Divergence, EdgeCaseRequirement, PatternArgument, Policy, SessionProfile, TypeEquality,
    TypePattern,
};

pub(crate) fn problems(policy: &Policy) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    if policy.schema_version != SCHEMA_VERSION {
        found.push(format!(
            "schema_version is {}, but this build reads version {SCHEMA_VERSION}",
            policy.schema_version
        ));
    }
    found.extend(
        policy
            .comparator
            .type_equality
            .iter()
            .flat_map(equality_problems),
    );
    if policy.session_profiles.is_empty() {
        found.push("session_profiles declares no profile".to_owned());
    }
    found.extend(
        policy
            .session_profiles
            .iter()
            .flat_map(|(name, profile)| profile_problems(name, profile)),
    );
    found.extend(
        policy
            .edge_cases
            .iter()
            .flat_map(|(name, requirement)| edge_case_problems(name, requirement)),
    );
    found.extend(
        policy
            .divergences
            .iter()
            .flat_map(|(name, divergence)| divergence_problems(name, divergence)),
    );
    found
}

fn variables(pattern: &TypePattern) -> BTreeSet<&str> {
    pattern
        .arguments
        .iter()
        .filter_map(|argument| match argument {
            PatternArgument::Variable(name) => Some(name.as_str()),
            PatternArgument::Literal(_) => None,
        })
        .collect()
}

fn equality_problems(equality: &TypeEquality) -> Vec<String> {
    let label = format!(
        "type_equality `{}` = `{}`",
        equality.snowflake, equality.duckdb
    );
    let mut found: Vec<String> = Vec::new();
    if equality.reason.trim().is_empty() {
        found.push(format!("{label} has no reason"));
    }
    match (
        parse_type_pattern(&equality.snowflake),
        parse_type_pattern(&equality.duckdb),
    ) {
        (Ok(snowflake), Ok(duckdb)) => {
            let bound = variables(&snowflake);
            let unbound: Vec<&str> = variables(&duckdb)
                .into_iter()
                .filter(|name| !bound.contains(name))
                .collect();
            if !unbound.is_empty() {
                found.push(format!(
                    "{label} uses parameters the Snowflake side does not bind: {}",
                    unbound.join(", ")
                ));
            }
        }
        (snowflake, duckdb) => {
            found.extend(snowflake.err().map(|error| format!("{label}: {error}")));
            found.extend(duckdb.err().map(|error| format!("{label}: {error}")));
        }
    }
    found
}

fn profile_problems(name: &str, profile: &SessionProfile) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    if !is_kebab_name(name) {
        found.push(format!(
            "session profile `{name}` needs a lower-case kebab name"
        ));
    }
    if profile.description.trim().is_empty() {
        found.push(format!("session profile `{name}` has no description"));
    }
    if profile.duckdb_time_zone.trim().is_empty() {
        found.push(format!("session profile `{name}` has no duckdb_time_zone"));
    }
    found.extend(
        profile
            .snowflake
            .keys()
            .filter(|key| !is_parameter_name(key))
            .map(|key| format!("session profile `{name}` has an invalid parameter name `{key}`")),
    );
    found.extend(
        profile
            .pinned
            .keys()
            .filter(|key| !is_parameter_name(key) || profile.snowflake.contains_key(*key))
            .map(|key| {
                format!(
                    "session profile `{name}` pins `{key}`, invalid or already a case parameter"
                )
            }),
    );
    found
}

fn edge_case_problems(name: &str, requirement: &EdgeCaseRequirement) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    if !is_kebab_name(name) {
        found.push(format!(
            "edge-case category `{name}` needs a lower-case kebab name"
        ));
    }
    if requirement.description.trim().is_empty() {
        found.push(format!("edge-case category `{name}` has no description"));
    }
    if requirement.required.is_empty() {
        found.push(format!("edge-case category `{name}` requires no input"));
    }
    let unique: BTreeSet<&String> = requirement.required.iter().collect();
    if unique.len() != requirement.required.len() {
        found.push(format!(
            "edge-case category `{name}` repeats a required input"
        ));
    }
    found
}

fn divergence_problems(name: &str, divergence: &Divergence) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    if divergence.constructs.is_empty() {
        found.push(format!("divergence `{name}` names no construct"));
    }
    found.extend(
        divergence
            .constructs
            .iter()
            .filter(|construct| !is_construct_key(construct))
            .map(|construct| {
                format!("divergence `{name}` has an invalid construct key `{construct}`")
            }),
    );
    if divergence.strict < divergence.default {
        found.push(format!(
            "divergence `{name}` is treated more leniently in strict mode"
        ));
    }
    if divergence.reason.trim().is_empty() {
        found.push(format!("divergence `{name}` has no reason"));
    }
    found
}

fn is_kebab_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

fn is_parameter_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
}

fn is_construct_key(key: &str) -> bool {
    const FAMILIES: [&str; 6] = ["node", "fn", "cast", "join", "clause", "syntax"];
    key.split_once(':')
        .is_some_and(|(family, name)| FAMILIES.contains(&family) && !name.is_empty())
}
