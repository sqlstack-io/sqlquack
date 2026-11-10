//! The user configuration schema. Every struct rejects unknown keys.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::policy::models::SessionValue;

/// A project's `sqlquack.toml`: each key changes a built-in default.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserConfig {
    /// The project's folders, session and Python runtime.
    #[serde(default)]
    pub project: ProjectConfig,
    /// Known differences by name: `true` allows one, `false` refuses one.
    #[serde(default)]
    pub allow: BTreeMap<String, bool>,
    /// The project's own DuckDB versions of Snowflake functions.
    #[serde(default)]
    pub replace: ReplaceConfig,
}

/// `[project]`: folders that differ from the defaults, the Snowflake session and the Python runtime.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    /// The folder of source YAML files, relative to the file; `sources` when absent.
    #[serde(default)]
    pub sources: Option<String>,
    /// The folder of SQL and Python function files, relative to the file; `functions` when absent.
    #[serde(default)]
    pub functions: Option<String>,
    /// Whether the project's Python functions run where the converted SQL runs.
    #[serde(default)]
    pub python_runtime: bool,
    /// Snowflake session parameters of the session the SQL runs in, as `--session` gives them.
    #[serde(default)]
    pub session: BTreeMap<String, SessionValue>,
}

/// `[replace]`: the folder of replacement functions and the ones checked by hash.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaceConfig {
    /// The folder of replacement files, relative to the file; `overrides` when absent.
    #[serde(default)]
    pub folder: Option<String>,
    /// Replacement name to the SHA-256 of the body it was checked for, as `--verified-override` gives it.
    #[serde(default)]
    pub verified: BTreeMap<String, String>,
}
