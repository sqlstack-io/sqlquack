//! Helper macros: versioned DuckDB macros in the `sqlquack` schema that converted SQL calls.

pub(crate) mod _helpers;
pub mod constants;
pub mod main;
pub mod models;

#[cfg(test)]
mod tests;
