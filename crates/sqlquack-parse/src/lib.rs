//! The one way to reach a parser: SQL parsed in the dialect it is written in.

#![forbid(unsafe_code)]

pub mod parsing;

#[cfg(feature = "bigquery")]
pub use sqlquack_syntax_bigquery as bigquery;
#[cfg(feature = "clickhouse")]
pub use sqlquack_syntax_clickhouse as clickhouse;
#[cfg(feature = "databricks")]
pub use sqlquack_syntax_databricks as databricks;
#[cfg(feature = "duckdb")]
pub use sqlquack_syntax_duckdb as duckdb;
#[cfg(feature = "postgres")]
pub use sqlquack_syntax_postgres as postgres;
pub use sqlquack_syntax_snowflake::{
    StrictFailure, TolerantParse, TolerantStatement, dialect_tokens, parse_expressions,
    parse_statements, parse_statements_with, parse_strict, parse_tolerant, tokenize,
};
#[cfg(feature = "sqlite")]
pub use sqlquack_syntax_sqlite as sqlite;
#[cfg(feature = "sqlserver")]
pub use sqlquack_syntax_sqlserver as sqlserver;
