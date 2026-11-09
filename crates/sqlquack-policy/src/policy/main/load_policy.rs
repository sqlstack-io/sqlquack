//! `load_policy`: read and validate a policy file.

use std::path::Path;

use crate::policy::_helpers::parsing::parse_policy;
use crate::policy::errors::PolicyError;
use crate::policy::models::Policy;

/// Reads `path` and validates it against the schema and every policy rule.
pub fn load_policy(path: &Path) -> Result<Policy, PolicyError> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| PolicyError::Read(format!("{}: {error}", path.display())))?;
    parse_policy(&text)
}
