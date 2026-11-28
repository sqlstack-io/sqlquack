//! What Python passes in and gets back.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sqlquack::cli::models::SourceDialect;

/// A report as `--json` prints it, with its diagnostics rendered as the command line shows them.
#[derive(Debug, Serialize)]
pub struct Rendered<T: Serialize> {
    /// The report.
    pub report: T,
    /// The diagnostics as text, as the command line prints them on standard error.
    pub rendered: String,
}

/// The options of `sqlquack convert`, as keyword arguments of `sqlquack.convert`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConvertRequest {
    /// `--dialect`; Snowflake when absent.
    #[serde(default)]
    pub dialect: SourceDialect,
    /// `--project`.
    pub project: Option<PathBuf>,
    /// Relations and functions declared in memory, in addition to the project's.
    pub schema: Option<serde_json::Value>,
    /// `--config`; the command line's default when absent.
    pub config: Option<PathBuf>,
    /// `--no-config`.
    pub no_config: bool,
    /// `--strict`.
    pub strict: bool,
    /// `--python-runtime`.
    pub python_runtime: bool,
    /// Known differences allowed (`true`) or refused (`false`) over the configuration's `[allow]`.
    pub allow: BTreeMap<String, bool>,
    /// Where those allowances are written, for help to name.
    pub allow_location: Option<String>,
    /// `--overrides`.
    pub overrides: Option<PathBuf>,
    /// `--verified-override NAME=SHA256`, by name.
    pub verified_overrides: BTreeMap<String, String>,
    /// `--session NAME=VALUE`, by name.
    pub session: BTreeMap<String, String>,
    /// `--compact`.
    pub compact: bool,
    /// `--no-bind-check`.
    pub no_bind_check: bool,
    /// `--max-call-depth`; the default when absent.
    pub max_call_depth: Option<usize>,
}
