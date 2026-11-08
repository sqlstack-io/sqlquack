//! Typing helpers: the types and nullability of values, per dialect.

pub(crate) mod duckdb_types;
pub(crate) mod duckdb_typing;
pub(crate) mod postgres_typing;
pub(crate) mod rules;
pub(crate) mod snowflake_sets;
pub(crate) mod snowflake_typing;
pub(crate) mod sqlite_typing;
