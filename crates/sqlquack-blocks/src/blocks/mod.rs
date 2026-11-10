//! DuckDB SQL fragments with Snowflake semantics, each recorded in conversion provenance.

pub(crate) mod _helpers;
pub mod constants;
pub mod main;
pub mod models;

#[cfg(test)]
mod tests;
