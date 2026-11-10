//! The project folder: its configuration, schema, session and overrides, under the caller's flags.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sqlquack_convert::snowflake::analysis::models::ConversionChoices;
use sqlquack_policy::config::constants::CONFIG_FILE_NAME;
use sqlquack_policy::config::main::load_config::load_config;
use sqlquack_policy::config::models::UserConfig;
use sqlquack_policy::policy::main::evidence_policy::evidence_policy;
use sqlquack_semantics::snowflake::typing::types::Schema;

use crate::cli::_helpers::evidence::conversion_choices;
use crate::cli::models::{PolicyArguments, ProjectChoices};
use crate::project::constants::OVERRIDES_FOLDER;
use crate::project::main::project_folder::project_folder;
use crate::project::main::project_schema::project_schema;

/// The project choices of `conversion.project` and its configuration, under the caller's flags.
pub(crate) fn project_choices(
    conversion: &PolicyArguments,
    (schema, session, overrides): (Option<&Schema>, &[String], Option<&Path>),
) -> Result<ProjectChoices, String> {
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
    let project_file = root.join(CONFIG_FILE_NAME);
    let config_file = match (conversion.no_config, conversion.config.clone()) {
        (true, _) => None,
        (false, Some(path)) if !path.is_file() => {
            return Err(format!(
                "no configuration file at {}; pass --config with an existing file, or leave it \
                 out to use the project's sqlquack.toml or, without one, the defaults",
                path.display()
            ));
        }
        (false, Some(path)) => Some(path),
        (false, None) => Some(project_file).filter(|path| path.is_file()),
    };
    let config = match &config_file {
        Some(path) => load_config(path, evidence_policy()).map_err(|error| error.to_string())?,
        None => UserConfig::default(),
    };
    let mut conversion = conversion.clone();
    conversion.python_runtime |= config.project.python_runtime;
    let given: BTreeSet<String> = session
        .iter()
        .filter_map(|setting| setting.split_once('='))
        .map(|(name, _)| name.trim().to_ascii_uppercase())
        .collect();
    let session: Vec<String> = config
        .project
        .session
        .iter()
        .filter(|(name, _)| !given.contains(&name.to_ascii_uppercase()))
        .map(|(name, value)| format!("{name}={}", value.text()))
        .chain(session.iter().cloned())
        .collect();
    let overrides = match overrides {
        Some(path) => Some(path.to_path_buf()),
        None => project_folder(&root, config.replace.folder.as_deref(), OVERRIDES_FOLDER)?,
    };
    let (schema, warnings) = project_schema(&root, &config.project, schema)?;
    let shown = config_file.as_ref().map(|_| match &conversion.config {
        Some(path) => path.clone(),
        None => PathBuf::from(format!("the project's {CONFIG_FILE_NAME}")),
    });
    let choices: ConversionChoices = conversion_choices(&conversion, (&config, shown.as_deref()));
    let verified_overrides: Vec<String> = config
        .replace
        .verified
        .iter()
        .map(|(name, hash)| format!("{name}={hash}"))
        .collect();
    Ok(ProjectChoices {
        schema,
        warnings,
        session,
        overrides,
        verified_overrides,
        conversion,
        choices,
    })
}
