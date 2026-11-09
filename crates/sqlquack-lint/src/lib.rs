//! SQL lint Rules over SQLQuack's tokens and syntax trees, ported from SQLClean's lint engine.

#![forbid(unsafe_code)]

#[cfg(feature = "bigquery")]
pub(crate) mod bigquery;
pub mod constants;
#[cfg(feature = "databricks")]
pub(crate) mod databricks;
pub(crate) mod dialects;
#[cfg(feature = "duckdb")]
pub(crate) mod duckdb;
pub mod lint;
#[cfg(feature = "postgres")]
pub(crate) mod postgres;
pub mod quality;
#[cfg(feature = "snowflake")]
pub(crate) mod snowflake;
#[cfg(feature = "sqlite")]
pub(crate) mod sqlite;
pub(crate) mod tree;

pub use crate::lint::main::batch_engine::lint_batch_json;
pub use crate::lint::main::catalogue::catalogue;
pub use crate::lint::main::cross_join_subject::cross_join_subject;
pub use crate::lint::main::default_rules::default_rules;
pub use crate::lint::main::engine::lint_json;
pub use crate::lint::main::fill_finding_path::fill_finding_path;
pub use crate::lint::main::lint::lint;
pub use crate::lint::main::parse_sql::parse_sql;
pub use crate::lint::main::validate_dialect::validate_dialect;
