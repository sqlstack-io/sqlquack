//! `duckdb_failure`: the kind of a DuckDB error message.

use crate::failures::constants::{DUCKDB_MESSAGE_KINDS, ERROR_TAG_PREFIX, SNOWFLAKE_ERRORS};
use crate::failures::models::DuckdbFailure;

/// A building block's tagged Snowflake error when the message has one, else DuckDB's own kind.
pub fn duckdb_failure(message: &str) -> Option<DuckdbFailure> {
    if let Some((_, tagged)) = message.split_once(ERROR_TAG_PREFIX) {
        let errno = tagged.split(':').next().unwrap_or_default();
        return SNOWFLAKE_ERRORS
            .iter()
            .find(|(code, _)| code.errno.to_string() == errno)
            .map(|(code, kind)| DuckdbFailure {
                kind: *kind,
                snowflake: Some(*code),
            });
    }
    DUCKDB_MESSAGE_KINDS
        .iter()
        .find(|(fragment, _)| message.contains(fragment))
        .map(|(_, kind)| DuckdbFailure {
            kind: *kind,
            snowflake: None,
        })
}
