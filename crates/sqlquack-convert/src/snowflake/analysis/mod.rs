//! The Snowflake conversion pipeline: parse, convert with the rules, or refuse.

pub(crate) mod _helpers;
pub mod constants;
pub mod main;
pub mod models;

#[cfg(test)]
mod tests;
