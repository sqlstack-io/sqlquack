//! Replay settings, and the Python functions a replay registers.

use std::path::PathBuf;

use serde::Serialize;

/// The DuckDB session a case replays in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplaySettings {
    /// DuckDB's `TimeZone`.
    pub time_zone: String,
    /// DuckDB's `default_null_order`.
    pub null_order: String,
}

/// A Python function a replay registers in DuckDB before its statement runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PythonFunction {
    /// The name the converted SQL calls.
    pub name: String,
    /// How values cross: `scalar-v1` or `variant-json-v1`.
    pub abi: String,
    /// The DuckDB parameter types, in order.
    pub arguments: Vec<String>,
    /// The DuckDB type of the result.
    pub returns: String,
    /// `volatile` or `immutable`.
    pub volatility: String,
    /// `called` or `returns_null`.
    pub null_input: String,
    /// The handler the source defines.
    pub handler: String,
    /// The Python source.
    pub source: String,
}

/// The Python functions a replay needs, and the interpreter that registers them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PythonReplay {
    /// A Python interpreter with the duckdb package SQLQuack links.
    pub interpreter: PathBuf,
    /// The functions.
    pub functions: Vec<PythonFunction>,
}
