//! `json_array_to_string`: ARRAY_TO_STRING of a VARIANT array.

use sqlquack_core::failures::constants::{NOT_AN_ARRAY_ARGUMENT, VARIANT_NOT_CAST};
use sqlquack_core::failures::main::tagged_message::tagged_message;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::raises::main::raised::raised;
use crate::semi_structured::_helpers::guards::bound;
use crate::semi_structured::_helpers::writing::{Root, sorted_text};
use crate::semi_structured::constants::{
    ARRAY_ELEMENT_NOT_TEXT_MESSAGE, JSON_ARRAY_TO_STRING, NOT_AN_ARRAY_ARGUMENT_MESSAGE,
};

/// The elements joined by `separator`: strings as text, other values as Snowflake writes JSON.
pub fn json_array_to_string((sql, separator): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_TO_STRING);
    let elements = "json_extract(a, '$[*]')";
    bound(
        sql,
        "a",
        &format!(
            "(CASE WHEN a IS NULL OR {separator} IS NULL THEN NULL \
             WHEN json_type(a) <> 'ARRAY' THEN CAST({} AS VARCHAR) \
             WHEN list_contains(list_transform({elements}, lambda e: json_type(e)), 'NULL') \
             THEN CAST({} AS VARCHAR) \
             ELSE array_to_string(list_transform({elements}, lambda e: CASE \
             WHEN json_type(e) = 'VARCHAR' THEN json_extract_string(e, '$') \
             ELSE {written} END), {separator}) END)",
            raised(&tagged_message(
                NOT_AN_ARRAY_ARGUMENT,
                NOT_AN_ARRAY_ARGUMENT_MESSAGE
            )),
            raised(&tagged_message(
                VARIANT_NOT_CAST,
                ARRAY_ELEMENT_NOT_TEXT_MESSAGE
            )),
            written = sorted_text("e", Root::Json),
        ),
    )
}
