//! `text_to_float`: text to FLOAT, accepting only spellings Snowflake is recorded to read.

use sqlquack_core::failures::constants::NUMBER_NOT_RECOGNIZED;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::_helpers::text::{conversion_failure, space_trimmed};
use crate::blocks::constants::{DECIMAL_TEXT_PATTERN, NON_FINITE_FLOAT_WORDS, TEXT_TO_FLOAT};
use crate::blocks::models::TextFailure;

/// Text to a double; unrecorded spellings fail or are NULL when trying, as is overflow when trying.
pub fn text_to_float(sql: &str, failure: TextFailure, log: &ProvenanceLog) -> String {
    log.building_block(TEXT_TO_FLOAT);
    let trimmed = space_trimmed(sql);
    let unread = format!(
        "CAST({} AS DOUBLE)",
        conversion_failure(
            failure,
            (NUMBER_NOT_RECOGNIZED, "Numeric value is not recognized")
        )
    );
    let words: String = NON_FINITE_FLOAT_WORDS
        .iter()
        .map(|(word, duckdb)| {
            format!(" WHEN lower({trimmed}) = '{word}' THEN CAST('{duckdb}' AS DOUBLE)")
        })
        .collect();
    let overflow = match failure {
        TextFailure::Null => format!(
            "(CASE WHEN isinf(CAST({trimmed} AS DOUBLE)) THEN {unread} ELSE CAST({trimmed} AS DOUBLE) END)"
        ),
        TextFailure::Text | TextFailure::Variant => format!("CAST({trimmed} AS DOUBLE)"),
    };
    format!(
        "(CASE WHEN {sql} IS NULL THEN NULL{words} \
         WHEN regexp_full_match({trimmed}, '{DECIMAL_TEXT_PATTERN}') THEN {overflow} \
         ELSE {unread} END)"
    )
}
