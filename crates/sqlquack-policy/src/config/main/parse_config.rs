//! `parse_config`: a user's `sqlquack.toml` text, checked against the evidence policy.

use crate::config::constants::ALLOW_TABLE;
use crate::config::main::check_allow::check_allow;
use crate::config::models::UserConfig;
use crate::policy::models::Policy;

/// The configuration `text` holds, whose `[allow]` may name only `evidence`'s known differences.
pub fn parse_config(text: &str, evidence: &Policy) -> Result<UserConfig, String> {
    let config: UserConfig =
        toml::from_str(text).map_err(|error| match text.contains("[divergences.") {
            true => format!(
                "{error}\nthis is a sqlquack.toml from before 0.4: list each known difference as \
             `name = true` or `false` under [{ALLOW_TABLE}]; `sqlquack explain --differences` \
             lists them"
            ),
            false => error.to_string(),
        })?;
    check_allow(
        &config.allow,
        evidence,
        (
            &format!("[{ALLOW_TABLE}]"),
            "set python_runtime = true under [project] when a Python runtime registers the \
             project's functions",
        ),
    )?;
    Ok(config)
}
