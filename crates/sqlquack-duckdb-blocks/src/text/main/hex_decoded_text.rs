//! `hex_decoded_text`: TRY_HEX_DECODE_STRING, the text whose UTF-8 bytes hexadecimal digits spell.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::text::constants::{HEX_DECODED_TEXT, HEX_PAIRS_PATTERN};

/// The decoded text, or NULL for an odd count, a non-hexadecimal digit or bytes not UTF-8.
pub fn hex_decoded_text(value: &str, log: &ProvenanceLog) -> String {
    log.building_block(HEX_DECODED_TEXT);
    format!(
        "list_transform([{value}], lambda hex_digits: CASE WHEN \
         regexp_full_match(hex_digits, '{HEX_PAIRS_PATTERN}') \
         THEN try(decode(unhex(hex_digits))) END)[1]"
    )
}
