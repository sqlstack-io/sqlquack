//! Snowflake's error kinds and rejections, and the tests that parse Snowflake SQL.

#[cfg(test)]
mod classification;
#[cfg(test)]
mod diagnostics;
pub mod failures;
pub mod refusals;

#[cfg(test)]
mod tests;
