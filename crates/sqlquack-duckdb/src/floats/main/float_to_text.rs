//! `float_to_text`: a FLOAT as the text Snowflake writes for it.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::floats::_helpers::values::bound;
use crate::floats::constants::{
    FLOAT_TEXT_FRACTION_DIGITS, FLOAT_TEXT_MAX_DIGITS, FLOAT_TEXT_MIN_DIGITS, FLOAT_TO_TEXT,
};

/// `%.{n}g` with n = max(10, min(15, whole digits + 9)), zero signed, as recorded.
pub fn float_to_text(value: &str, log: &ProvenanceLog) -> String {
    log.building_block(FLOAT_TO_TEXT);
    let sql = "f";
    let bands: String = (FLOAT_TEXT_MIN_DIGITS..FLOAT_TEXT_MAX_DIGITS)
        .map(|digits| {
            let whole = digits - FLOAT_TEXT_FRACTION_DIGITS;
            format!("WHEN abs({sql}) < 1e{whole} THEN printf('%.{digits}g', {sql}) ")
        })
        .collect();
    let body = format!(
        "(CASE WHEN {sql} IS NULL THEN NULL WHEN isnan({sql}) THEN 'NaN' \
         WHEN isinf({sql}) THEN CASE WHEN {sql} > 0 THEN 'inf' ELSE '-inf' END \
         WHEN {sql} = 0 THEN CASE WHEN signbit({sql}) THEN '-0' ELSE '0' END \
         {bands}ELSE printf('%.{FLOAT_TEXT_MAX_DIGITS}g', {sql}) END)"
    );
    bound(value, sql, &body)
}
