//! The instruction-count guard: callgrind's counts for strict Snowflake parsing and for analysis.

pub(crate) mod _helpers;
pub mod constants;
pub mod main;
pub mod models;

#[cfg(test)]
mod tests;
