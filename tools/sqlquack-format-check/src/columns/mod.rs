//! The output columns DuckDB binds a query to, before and after the formatter lays it out.

pub(crate) mod _helpers;
pub mod constants;
pub mod main;
pub mod models;

#[cfg(test)]
mod tests;
