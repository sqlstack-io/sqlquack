//! Snowflake to DuckDB: the rule catalogue, the converter and the pipeline that runs them.

pub mod analysis;
pub mod catalogue;
pub mod conversion;
pub mod session;
pub(crate) mod text_formats;
