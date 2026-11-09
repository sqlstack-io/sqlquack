//! The project folder: its policy, schema, session, confirmations and overrides, under the caller's flags.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sqlquack_policy::policy::constants::POLICY_FILE_NAME;
use sqlquack_policy::policy::main::load_project_file::load_project_file;
use sqlquack_policy::policy::models::{Policy, ProjectPolicy};
use sqlquack_semantics::snowflake::types::models::Schema;

use crate::cli::_helpers::evidence::policy_file;
use crate::cli::models::{PolicyArguments, ProjectChoices};
use crate::project::constants::OVERRIDES_FOLDER;
use crate::project::main::project_folder::project_folder;
use crate::project::main::project_schema::project_schema;

/// The policy and the project choices of `conversion.project`, under the caller's flags.
pub(crate) fn project_choices(
    conversion: &PolicyArguments,
    (schema, session, overrides): (Option<&Schema>, &[String], Option<&Path>),
) -> Result<(Option<Policy>, ProjectChoices), String> {
    let root = conversion
        .project
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));
    if !root.is_dir() {
        return Err(format!(
            "the project folder {} does not exist",
            root.display()
        ));
    }
    let (file_policy, project) = project_file(&root.join(POLICY_FILE_NAME))?;
    let policy = match (conversion.builtin_policy, conversion.policy.as_deref()) {
        (true, _) => None,
        (false, Some(path)) => Some(policy_file(path)?),
        (false, None) => file_policy,
    };
    let mut conversion = conversion.clone();
    conversion.python_runtime |= project.confirm.python_runtime;
    conversion.utc_session |= project.confirm.utc_session;
    conversion.confirm_declared_offsets |= project.confirm.declared_offsets;
    let given: BTreeSet<String> = session
        .iter()
        .filter_map(|setting| setting.split_once('='))
        .map(|(name, _)| name.trim().to_ascii_uppercase())
        .collect();
    let session: Vec<String> = project
        .session
        .iter()
        .filter(|(name, _)| !given.contains(&name.to_ascii_uppercase()))
        .map(|(name, value)| format!("{name}={}", value.text()))
        .chain(session.iter().cloned())
        .collect();
    let overrides = match overrides {
        Some(path) => Some(path.to_path_buf()),
        None => project_folder(&root, project.overrides.as_deref(), OVERRIDES_FOLDER)?,
    };
    let (schema, warnings) = project_schema(&root, &project, schema)?;
    Ok((
        policy,
        ProjectChoices {
            schema,
            warnings,
            session,
            overrides,
            conversion,
        },
    ))
}

/// The policy and `[project]` of the project's `sqlquack.toml`, or neither when it has none.
fn project_file(path: &Path) -> Result<(Option<Policy>, ProjectPolicy), String> {
    match path.is_file() {
        true => load_project_file(path).map_err(|error| error.to_string()),
        false => Ok((None, ProjectPolicy::default())),
    }
}
