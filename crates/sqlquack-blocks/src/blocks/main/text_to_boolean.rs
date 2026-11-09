//! `text_to_boolean`: text to BOOLEAN, accepting only spellings Snowflake is recorded to read.

use sqlquack_diagnostics::failures::constants::BOOLEAN_NOT_RECOGNIZED;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::_helpers::text::{conversion_failure, once_bound, space_trimmed, word_list};
use crate::blocks::constants::{FALSE_WORDS, TEXT_TO_BOOLEAN, TRUE_WORDS};
use crate::blocks::models::TextFailure;

/// Text to a boolean, ignoring case and surrounding spaces; other text fails, or is NULL when trying.
pub fn text_to_boolean(sql: &str, failure: TextFailure, log: &ProvenanceLog) -> String {
    log.building_block(TEXT_TO_BOOLEAN);
    once_bound(sql, "boolean_text", |sql| read_boolean(sql, failure))
}

/// The CASE reading the text `sql` as a boolean.
fn read_boolean(sql: &str, failure: TextFailure) -> String {
    let word = format!("lower({})", space_trimmed(sql));
    format!(
        "(CASE WHEN {sql} IS NULL THEN NULL WHEN {word} IN ({}) THEN TRUE \
         WHEN {word} IN ({}) THEN FALSE ELSE CAST({} AS BOOLEAN) END)",
        word_list(TRUE_WORDS),
        word_list(FALSE_WORDS),
        conversion_failure(
            failure,
            (BOOLEAN_NOT_RECOGNIZED, "Boolean value is not recognized")
        )
    )
}
