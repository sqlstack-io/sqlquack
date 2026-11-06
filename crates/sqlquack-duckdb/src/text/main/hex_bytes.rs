//! `hex_bytes`: text of whole pairs of hexadecimal digits to BINARY, as under BINARY_INPUT_FORMAT HEX.

use sqlquack_core::failures::constants::HEX_DECODE_FAILED;
use sqlquack_core::failures::main::tagged_message::tagged_message;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::raises::main::raised::raised;
use crate::text::constants::{HEX_BYTES, HEX_BYTES_MESSAGE, HEX_PAIRS_PATTERN};

/// The bytes hex digits of either case spell; other text raises 100115, or is NULL when `trying`.
pub fn hex_bytes(sql: &str, trying: bool, log: &ProvenanceLog) -> String {
    log.building_block(HEX_BYTES);
    let failure = match trying {
        true => "NULL".to_owned(),
        false => raised(&tagged_message(HEX_DECODE_FAILED, HEX_BYTES_MESSAGE)),
    };
    format!(
        "list_transform([{sql}], lambda hex_text: CASE WHEN hex_text IS NULL THEN NULL \
         WHEN regexp_full_match(hex_text, '{HEX_PAIRS_PATTERN}') THEN unhex(hex_text) \
         ELSE CAST({failure} AS BLOB) END)[1]"
    )
}
