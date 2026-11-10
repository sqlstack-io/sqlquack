//! `hex_text`: HEX_ENCODE, the hexadecimal digits of a text's UTF-8 bytes.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::text::constants::HEX_TEXT;

/// Two hexadecimal digits per UTF-8 byte, upper-case unless `lower`.
pub fn hex_text(value: &str, lower: bool, log: &ProvenanceLog) -> String {
    log.building_block(HEX_TEXT);
    match lower {
        true => format!("lower(HEX({value}))"),
        false => format!("hex({value})"),
    }
}
