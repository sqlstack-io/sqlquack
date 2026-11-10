//! A known difference described for users.

use crate::policy::models::DivergenceFailure;

/// One `[divergences.<name>]` entry described in plain English for messages, explain and docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnownDifference {
    /// The policy's divergence name, such as `hash`.
    pub name: &'static str,
    /// A short title, such as "HASH gives a different value in DuckDB".
    pub title: &'static str,
    /// What differs between Snowflake and the converted SQL.
    pub differs: &'static str,
    /// When relying on it gives the same result as Snowflake.
    pub fine_when: &'static str,
    /// When relying on it does not give Snowflake's result, and what happens instead.
    pub wrong_when: &'static str,
    /// Whether relying on it can return a different value, or only raise an error.
    pub failure: DivergenceFailure,
}

impl KnownDifference {
    /// How it fails, in plain words: "a different value" or "an error".
    pub fn fails_with(&self) -> &'static str {
        match self.failure {
            DivergenceFailure::Value => "a different value",
            DivergenceFailure::Loud => "an error",
        }
    }
}
