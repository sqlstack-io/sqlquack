//! `parse_config`: a user's `sqlquack.toml` text, checked against the evidence policy.

use crate::config::constants::ALLOW_TABLE;
use crate::config::models::UserConfig;
use crate::policy::constants::PROJECT_UDF;
use crate::policy::main::nearest_name::nearest_name;
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
    let runtime = config.allow.contains_key(PROJECT_UDF).then(|| {
        format!(
            "[{ALLOW_TABLE}] cannot allow `{PROJECT_UDF}`: set python_runtime = true under \
                 [project] when a Python runtime registers the project's functions"
        )
    });
    let unknown: Vec<String> = config
        .allow
        .keys()
        .filter(|name| !evidence.divergences.contains_key(*name))
        .map(|name| {
            let known = evidence.divergences.keys().map(String::as_str);
            match nearest_name(name, known) {
                Some(near) => format!(
                    "[{ALLOW_TABLE}] names no known difference `{name}`; did you mean `{near}`?"
                ),
                None => format!(
                    "[{ALLOW_TABLE}] names no known difference `{name}`; `sqlquack explain \
                     --differences` lists them"
                ),
            }
        })
        .chain(runtime)
        .collect();
    match unknown.is_empty() {
        true => Ok(config),
        false => Err(unknown.join("; ")),
    }
}
