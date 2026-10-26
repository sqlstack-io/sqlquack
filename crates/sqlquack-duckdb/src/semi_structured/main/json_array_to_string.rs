//! `json_array_to_string`: ARRAY_TO_STRING of a VARIANT array.

use sqlquack_core::failures::constants::{NOT_AN_ARRAY_ARGUMENT, VARIANT_NOT_CAST};
use sqlquack_core::failures::main::tagged_message::tagged_message;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::main::raised::raised;
use crate::semi_structured::_helpers::guards::{bound, fidelity_error, keys_unsorted};
use crate::semi_structured::constants::{
    ARRAY_ELEMENT_NOT_TEXT_MESSAGE, JSON_ARRAY_TO_STRING, NOT_AN_ARRAY_ARGUMENT_MESSAGE,
};

/// The elements joined by `separator`, strings as text and other values as JSON, or the error.
pub fn json_array_to_string((sql, separator): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_TO_STRING);
    let elements = "json_extract(a, '$[*]')";
    bound(
        sql,
        "a",
        &format!(
            "(CASE WHEN a IS NULL OR json_type(a) = 'NULL' OR {separator} IS NULL THEN NULL \
             WHEN json_type(a) <> 'ARRAY' THEN CAST({} AS VARCHAR) \
             WHEN list_contains(list_transform({elements}, lambda e: json_type(e)), 'NULL') \
             THEN CAST({} AS VARCHAR) \
             WHEN {} THEN CAST({} AS VARCHAR) \
             ELSE array_to_string(list_transform({elements}, lambda e: CASE json_type(e) \
             WHEN 'VARCHAR' THEN json_extract_string(e, '$') ELSE CAST(e AS VARCHAR) END), \
             {separator}) END)",
            raised(&tagged_message(
                NOT_AN_ARRAY_ARGUMENT,
                NOT_AN_ARRAY_ARGUMENT_MESSAGE
            )),
            raised(&tagged_message(
                VARIANT_NOT_CAST,
                ARRAY_ELEMENT_NOT_TEXT_MESSAGE
            )),
            keys_unsorted("a"),
            fidelity_error("an object whose keys Snowflake writes in another order")
        ),
    )
}
