//! Policy text to a validated policy.

use crate::policy::_helpers::validation::problems;
use crate::policy::errors::PolicyError;
use crate::policy::models::Policy;

pub(crate) fn parse_policy(text: &str) -> Result<Policy, PolicyError> {
    let policy: Policy =
        toml::from_str(text).map_err(|error| PolicyError::Schema(error.to_string()))?;
    let found = problems(&policy);
    if found.is_empty() {
        Ok(policy)
    } else {
        Err(PolicyError::Invalid(found))
    }
}
