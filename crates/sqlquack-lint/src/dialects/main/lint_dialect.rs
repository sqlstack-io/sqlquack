//! `lint_dialect`: the dialect a name names, as lint reads it.

use crate::lint::constants::UNPARSED_DIALECTS;
use crate::lint::models::LintDialect;

/// The SQLQuack dialect `name` names; `None` for a known dialect SQLQuack does not parse yet.
pub(crate) fn lint_dialect(name: &str) -> Result<Option<LintDialect>, String> {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        #[cfg(feature = "snowflake")]
        "snowflake" => Ok(Some(LintDialect::Snowflake)),
        #[cfg(not(feature = "snowflake"))]
        "snowflake" => Err(format!(
            "{}: snowflake",
            crate::lint::constants::UNBUILT_DIALECT_FAILURE
        )),
        #[cfg(feature = "duckdb")]
        "duckdb" => Ok(Some(LintDialect::DuckDB)),
        #[cfg(not(feature = "duckdb"))]
        "duckdb" => Err(format!(
            "{}: duckdb",
            crate::lint::constants::UNBUILT_DIALECT_FAILURE
        )),
        #[cfg(feature = "sqlite")]
        "sqlite" => Ok(Some(LintDialect::SQLite)),
        #[cfg(not(feature = "sqlite"))]
        "sqlite" => Err(format!(
            "{}: sqlite",
            crate::lint::constants::UNBUILT_DIALECT_FAILURE
        )),
        #[cfg(feature = "postgres")]
        "postgres" | "postgresql" | "generic" | "" => Ok(Some(LintDialect::PostgreSQL)),
        #[cfg(not(feature = "postgres"))]
        "postgres" | "postgresql" | "generic" | "" => Err(format!(
            "{}: postgres",
            crate::lint::constants::UNBUILT_DIALECT_FAILURE
        )),
        #[cfg(feature = "bigquery")]
        "bigquery" => Ok(Some(LintDialect::BigQuery)),
        #[cfg(not(feature = "bigquery"))]
        "bigquery" => Err(format!(
            "{}: bigquery",
            crate::lint::constants::UNBUILT_DIALECT_FAILURE
        )),
        #[cfg(feature = "databricks")]
        "databricks" => Ok(Some(LintDialect::Databricks)),
        #[cfg(not(feature = "databricks"))]
        "databricks" => Err(format!(
            "{}: databricks",
            crate::lint::constants::UNBUILT_DIALECT_FAILURE
        )),
        _ if UNPARSED_DIALECTS.contains(&lower.as_str()) => Ok(None),
        _ => Err(format!("Unknown dialect: {name}")),
    }
}
