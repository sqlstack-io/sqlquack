//! What Python passes in and gets back.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
    /// `--overrides`.
    pub overrides: Option<PathBuf>,
    /// `--verified-override NAME=SHA256`, by name.
    pub verified_overrides: BTreeMap<String, String>,
    /// `--session NAME=VALUE`, by name.
    pub session: BTreeMap<String, String>,
    /// `--compact`.
    pub compact: bool,
}
