//! Policy text to a validated policy.

use crate::policy::_helpers::validation::problems;
use crate::policy::constants::DIVERGENCE_FIELDS;
use crate::policy::errors::PolicyError;
use crate::policy::main::nearest_name::nearest_name;
use crate::policy::models::Policy;

pub(crate) fn parse_policy(text: &str) -> Result<Policy, PolicyError> {
    let policy: Policy = toml::from_str(text).map_err(|error| {
        PolicyError::Schema(match text.parse::<toml::Table>() {
            Ok(table) => incomplete_divergence(&table).unwrap_or_else(|| error.to_string()),
            Err(_not_toml) => error.to_string(),
        })
    })?;
    let found = problems(&policy);
    if found.is_empty() {
        Ok(policy)
    } else {
        Err(PolicyError::Invalid(found))
    }
}

/// Why a `[divergences.<id>]` table lacking fields fails: a misspelt id, or a partial entry.
fn incomplete_divergence(table: &toml::Table) -> Option<String> {
    let divergences = table.get("divergences")?.as_table()?;
    let missing = |entry: &toml::Value| -> Vec<&str> {
        DIVERGENCE_FIELDS
            .iter()
            .copied()
            .filter(|field| entry.get(field).is_none())
            .collect()
    };
    let (id, absent) = divergences
        .iter()
        .map(|(id, entry)| (id, missing(entry)))
        .find(|(_, absent)| !absent.is_empty())?;
    let nearest = nearest_name(
        id,
        divergences
            .iter()
            .filter(|(other, entry)| *other != id && missing(entry).is_empty())
            .map(|(other, _)| other.as_str()),
    );
    Some(match nearest {
        Some(known) => {
            format!("unknown divergence `{id}` in [divergences.{id}]; did you mean `{known}`?")
        }
        None => format!(
            "[divergences.{id}] is missing field{} {}: a policy file declares each divergence in \
             full, so copy its entry from SQLQuack's evidence.toml, or check the id's \
             spelling",
            if absent.len() == 1 { "" } else { "s" },
            absent
                .iter()
                .map(|field| format!("`{field}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    })
}
