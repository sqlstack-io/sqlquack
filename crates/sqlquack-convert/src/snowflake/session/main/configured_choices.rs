//! `configured_choices`: a user's `sqlquack.toml` text as conversion reads it, without a file.

use std::collections::BTreeMap;

use crate::snowflake::analysis::models::ConversionChoices;
use crate::snowflake::session::main::parse_session::parse_session;
use sqlquack_policy::config::constants::CONFIG_FILE_NAME;
use sqlquack_policy::config::main::parse_config::parse_config;
use sqlquack_policy::policy::main::evidence_policy::evidence_policy;

/// The choices of config `text` under `strict`, and its session with `parameters` overriding it.
pub fn configured_choices(
    text: &str,
    strict: bool,
    parameters: &BTreeMap<String, String>,
) -> Result<(ConversionChoices, BTreeMap<String, String>), String> {
    let config = parse_config(text, evidence_policy())?;
    let settings: Vec<String> = config
        .project
        .session
        .iter()
        .map(|(name, value)| format!("{name}={}", value.text()))
        .collect();
    let mut session = parse_session(&settings)?;
    session.extend(parameters.clone());
    let choices = ConversionChoices {
        config_file: Some(CONFIG_FILE_NAME.to_owned()),
        allow: config.allow,
        strict,
        python_runtime: config.project.python_runtime,
    };
    Ok((choices, session))
}
