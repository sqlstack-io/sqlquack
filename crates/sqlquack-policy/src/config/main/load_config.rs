//! `load_config`: read a user's `sqlquack.toml` and check it against the evidence policy.

use std::path::Path;

use crate::config::errors::ConfigError;
use crate::config::main::parse_config::parse_config;
use crate::config::models::UserConfig;
use crate::policy::models::Policy;

/// The configuration in `path`, whose `[allow]` may name only `evidence`'s known differences.
pub fn load_config(path: &Path, evidence: &Policy) -> Result<UserConfig, ConfigError> {
    let failed = |message: String| ConfigError {
        path: path.display().to_string(),
        message,
    };
    let text = std::fs::read_to_string(path).map_err(|error| failed(error.to_string()))?;
    parse_config(&text, evidence).map_err(failed)
}
