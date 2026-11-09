//! Helpers of the dialect profiles.

#[cfg(feature = "native-parsers")]
pub(crate) mod parse_errors;
#[cfg(not(all(
    feature = "bigquery",
    feature = "databricks",
    feature = "duckdb",
    feature = "postgres",
    feature = "snowflake",
    feature = "sqlite"
)))]
pub(crate) mod unbuilt_profile;
