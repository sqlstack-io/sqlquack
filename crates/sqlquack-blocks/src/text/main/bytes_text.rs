//! `bytes_text`: bytes held as hexadecimal digits, read as UTF-8 text.

use sqlquack_diagnostics::failures::constants::INVALID_UTF8;
use sqlquack_diagnostics::failures::main::tagged_message::tagged_message;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::raises::main::raised::raised;
use crate::text::constants::{BYTES_TEXT, INVALID_UTF8_MESSAGE};

/// The text the bytes spell, NUL characters kept; bytes that are not UTF-8 raise 100146.
pub fn bytes_text(value: &str, log: &ProvenanceLog) -> String {
    log.building_block(BYTES_TEXT);
    let invalid = raised(&tagged_message(INVALID_UTF8, INVALID_UTF8_MESSAGE));
    format!(
        "list_transform([{value}], lambda byte_digits: CASE WHEN byte_digits IS NULL THEN NULL \
         ELSE coalesce(try(decode(unhex(byte_digits))), CAST({invalid} AS VARCHAR)) END)[1]"
    )
}
