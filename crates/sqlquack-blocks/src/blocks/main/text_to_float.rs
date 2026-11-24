//! `text_to_float`: text to FLOAT, accepting only spellings Snowflake is recorded to read.

use sqlquack_diagnostics::failures::constants::NUMBER_NOT_RECOGNIZED;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::_helpers::text::{conversion_failure, once_bound, space_trimmed};
use crate::blocks::constants::{DECIMAL_TEXT_PATTERN, NON_FINITE_FLOAT_WORDS, TEXT_TO_FLOAT};
use crate::blocks::models::TextFailure;

/// Text to a double; special-value words in ASCII only; other unrecorded text fails, or is NULL trying.
pub fn text_to_float(sql: &str, failure: TextFailure, log: &ProvenanceLog) -> String {
    log.building_block(TEXT_TO_FLOAT);
    once_bound(sql, "float_text", |sql| match failure {
        TextFailure::Variant => released_read_float(sql),
        TextFailure::Text | TextFailure::Null => read_float(sql, failure),
    })
}

/// The CASE reading the text `sql` as a double, its trimmed text bound once.
fn read_float(sql: &str, failure: TextFailure) -> String {
    let trimmed = "float_trimmed";
    let unread = conversion_failure(
        failure,
        (NUMBER_NOT_RECOGNIZED, "Numeric value is not recognized"),
    );
    let infinite = "(?i:[+-]?inf|-?infinity)";
    let finite = match failure {
        TextFailure::Null | TextFailure::Variant => format!(
            "WHEN regexp_full_match({trimmed}, '{infinite}') THEN CAST({trimmed} AS DOUBLE) \
             WHEN regexp_full_match({trimmed}, '{DECIMAL_TEXT_PATTERN}') THEN \
             (CASE WHEN isinf(CAST({trimmed} AS DOUBLE)) THEN NULL ELSE CAST({trimmed} AS DOUBLE) END)"
        ),
        TextFailure::Text => format!(
            "WHEN regexp_full_match({trimmed}, '{DECIMAL_TEXT_PATTERN}|{infinite}') \
             THEN CAST({trimmed} AS DOUBLE)"
        ),
    };
    format!(
        "(CASE WHEN {sql} IS NULL THEN NULL ELSE list_transform([{}], lambda {trimmed}: \
         CASE WHEN regexp_full_match({trimmed}, '(?i:-?nan)') THEN CAST('nan' AS DOUBLE) \
         {finite} ELSE {unread} END)[1] END)",
        space_trimmed(sql)
    )
}

/// The CASE reading the text `sql` as a double in released variant_float_v1, whose body is pinned.
fn released_read_float(sql: &str) -> String {
    let trimmed = space_trimmed(sql);
    let unread = format!(
        "CAST({} AS DOUBLE)",
        conversion_failure(
            TextFailure::Variant,
            (NUMBER_NOT_RECOGNIZED, "Numeric value is not recognized")
        )
    );
    let words: String = NON_FINITE_FLOAT_WORDS
        .iter()
        .map(|(word, duckdb)| {
            format!(
                " WHEN lower({trimmed}) = '{word}' AND strlen({trimmed}) = length({trimmed}) \
                 THEN CAST('{duckdb}' AS DOUBLE)"
            )
        })
        .collect();
    format!(
        "(CASE WHEN {sql} IS NULL THEN NULL{words} \
         WHEN regexp_full_match({trimmed}, '{DECIMAL_TEXT_PATTERN}') THEN CAST({trimmed} AS DOUBLE) \
         ELSE {unread} END)"
    )
}
