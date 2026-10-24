//! `parse_policy`: policy text to a validated policy.

use crate::policy::_helpers::validation::problems;
use crate::policy::errors::PolicyError;
use crate::policy::models::Policy;

/// Parses policy TOML, rejecting unknown keys, then checks every validation rule.
pub fn parse_policy(text: &str) -> Result<Policy, PolicyError> {
    let policy: Policy =
        toml::from_str(text).map_err(|error| PolicyError::Schema(error.to_string()))?;
    let found = problems(&policy);
    if found.is_empty() {
        Ok(policy)
    } else {
        Err(PolicyError::Invalid(found))
    }
}
