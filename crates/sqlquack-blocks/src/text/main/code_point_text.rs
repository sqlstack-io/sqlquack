//! `code_point_text`: CHR, the character with a code point.

use sqlquack_diagnostics::failures::constants::INVALID_CHARACTER_CODE;
use sqlquack_diagnostics::failures::main::tagged_message::tagged_message;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::raises::main::raised::raised;
use crate::text::constants::{
    CHARACTER_CODE_MESSAGE, CODE_POINT_TEXT, MAX_CODE_POINT, SURROGATE_CODE_POINTS,
};

/// The character with the integer code point; negative, surrogate or too large raise 100062.
pub fn code_point_text(value: &str, log: &ProvenanceLog) -> String {
    log.building_block(CODE_POINT_TEXT);
    let code = "n";
    let invalid = raised(&tagged_message(
        INVALID_CHARACTER_CODE,
        CHARACTER_CODE_MESSAGE,
    ));
    let (low, high) = SURROGATE_CODE_POINTS;
    let body = format!(
        "(CASE WHEN {code} IS NULL THEN NULL \
         WHEN {code} < 0 OR {code} > {MAX_CODE_POINT} OR {code} BETWEEN {low} AND {high} \
         THEN CAST({invalid} AS VARCHAR) ELSE chr(CAST({code} AS INTEGER)) END)"
    );
    format!("list_transform([{value}], lambda {code}: {body})[1]")
}
