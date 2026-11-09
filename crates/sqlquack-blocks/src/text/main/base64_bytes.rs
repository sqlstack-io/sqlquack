//! `base64_bytes`: TRY_BASE64_DECODE_BINARY, its bytes as upper-case hexadecimal digits.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::text::constants::{BASE64_BYTES, BASE64_PATTERN};

/// The bytes, line feeds anywhere ignored; NULL for any other text Snowflake does not decode, as recorded.
pub fn base64_bytes(value: &str, log: &ProvenanceLog) -> String {
    log.building_block(BASE64_BYTES);
    format!(
        "list_transform([replace({value}, chr(10), '')], lambda base64_text: CASE WHEN \
         regexp_full_match(base64_text, '{BASE64_PATTERN}') \
         THEN hex(from_base64(base64_text)) END)[1]"
    )
}
