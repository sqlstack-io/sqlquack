//! Syntax tests ported from polyglot's integration tests.

pub(crate) mod compat;
mod data_type_api;
mod deep_nesting_regression;
mod duckdb_json_arrow_context;
mod duckdb_unreserved_relation_aliases;
mod error_handling;
mod numeric_exponent_boundaries;
mod snowflake_regression_test;
mod typed_expression;
