//! The project beside the policy file: its schema, session, confirmations and overrides, under the caller's flags.

use std::collections::BTreeSet;
use std::path::Path;

use sqlquack_policy::policy::models::Policy;
use sqlquack_semantics::types::models::Schema;

use crate::cli::models::{PolicyArguments, ProjectChoices};
use crate::project::constants::OVERRIDES_FOLDER;
use crate::project::main::load_schema::load_schema;
use crate::project::main::project_folder::project_folder;
use crate::project::main::project_schema::project_schema;

/// The project choices for `policy`, read from `conversion.policy`, under the caller's flags.
pub(crate) fn project_choices(
    policy: Option<&Policy>,
    conversion: &PolicyArguments,
    (schema, session, overrides): (Option<&Path>, &[String], Option<&Path>),
) -> Result<ProjectChoices, String> {
    let file = schema.map(schema_file).transpose()?;
    let mut conversion = conversion.clone();
    let Some(policy) = policy else {
        return Ok(ProjectChoices {
            schema: file.map(|(_, schema)| schema).unwrap_or_default(),
            session: session.to_vec(),
            overrides: overrides.map(Path::to_path_buf),
            conversion,
        });
    };
    let root = conversion
        .policy
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let project = &policy.project;
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
    Ok(ProjectChoices {
        schema: project_schema(&root, project, file)?,
        session,
        overrides,
        conversion,
    })
}

/// The schema file at `path`, kept with its path to name it in a clash.
fn schema_file(path: &Path) -> Result<(&Path, Schema), String> {
    load_schema(path).map(|schema| (path, schema))
}
