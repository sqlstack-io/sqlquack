//! Converting one Databricks SQL statement to DuckDB, rule by rule, or refusing.

pub(crate) mod _helpers;
pub mod constants;
pub mod main;
pub mod models;
#[cfg(test)]
mod tests;
