//! `load_project_file`: a project's `sqlquack.toml`, which may hold a policy, `[project]`, or both.

use std::path::Path;

use crate::policy::_helpers::parsing::parse_policy;
use crate::policy::constants::PROJECT_TABLE;
use crate::policy::errors::PolicyError;
use crate::policy::models::{Policy, ProjectPolicy};

/// The policy `path` holds, or None when it holds only `[project]`, and its project settings.
pub fn load_project_file(path: &Path) -> Result<(Option<Policy>, ProjectPolicy), PolicyError> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| PolicyError::Read(format!("{}: {error}", path.display())))?;
    let table: toml::Table = text
        .parse()
        .map_err(|error: toml::de::Error| PolicyError::Schema(error.to_string()))?;
    if table.keys().any(|key| key != PROJECT_TABLE) {
        let policy = parse_policy(&text)?;
        let project = policy.project.clone();
        return Ok((Some(policy), project));
    }
    let project = match table.get(PROJECT_TABLE) {
        Some(value) => value
            .clone()
            .try_into::<ProjectPolicy>()
            .map_err(|error| PolicyError::Schema(format!("[{PROJECT_TABLE}]: {error}")))?,
        None => ProjectPolicy::default(),
    };
    Ok((None, project))
}
