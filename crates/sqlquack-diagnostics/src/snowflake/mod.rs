//! Snowflake's error kinds, rejections and cast type names, and the tests that parse Snowflake SQL.

pub mod classification;
#[cfg(test)]
mod diagnostics;
pub mod failures;
pub mod refusals;

#[cfg(test)]
mod tests;
