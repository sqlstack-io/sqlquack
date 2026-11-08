//! DuckDB 2.0.0's PEG grammar, by recursive descent, building sqlquack-syntax trees.

pub(crate) mod _helpers;
pub(crate) mod constants;
pub mod errors;
pub mod main;

#[cfg(test)]
mod tests;
