//! `float_to_text`: a FLOAT as the text Snowflake writes for it.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::floats::_helpers::values::bound;
use crate::floats::constants::{
    FLOAT_TEXT, FLOAT_TEXT_LONG_DIGITS, FLOAT_TEXT_LONG_RANGES, FLOAT_TEXT_SHORT_DIGITS,
    FLOAT_TEXT_SHORT_RANGES, FLOAT_TO_TEXT,
};
use crate::raises::main::raised::raised;

/// The recorded text: up to ten significant digits as written, more only in recorded magnitudes.
pub fn float_to_text(value: &str, log: &ProvenanceLog) -> String {
    log.building_block(FLOAT_TO_TEXT);
    let sql = "f";
    let unproven = raised(&format!(
        "{FLOAT_TEXT}: a FLOAT whose text no recording shows: negative zero, or more than ten \
         significant digits at a magnitude whose text is unrecorded"
    ));
    let text = |digits: u8| format!("printf('%.{digits}g', {sql})");
    let full = text(FLOAT_TEXT_LONG_DIGITS);
    let digits = format!(
        "length(ltrim(regexp_replace(regexp_replace({full}, 'e.*$', ''), '[^0-9]', '', 'g'), '0'))"
    );
    let within = |ranges: &[(&str, &str)]| {
        ranges
            .iter()
            .map(|(from, to)| {
                format!(
                    "abs({sql}) >= CAST('{from}' AS DOUBLE) AND abs({sql}) < CAST('{to}' AS DOUBLE)"
                )
            })
            .collect::<Vec<String>>()
            .join(" OR ")
    };
    let body = format!(
        "(CASE WHEN {sql} IS NULL THEN NULL WHEN isnan({sql}) THEN 'NaN' \
         WHEN isinf({sql}) THEN CASE WHEN {sql} > 0 THEN 'inf' ELSE '-inf' END \
         WHEN {sql} = 0 AND NOT signbit({sql}) THEN '0' \
         WHEN {sql} = 0 THEN CAST({unproven} AS VARCHAR) \
         WHEN {digits} <= {FLOAT_TEXT_SHORT_DIGITS} THEN {full} \
         WHEN {} THEN {} \
         WHEN {} THEN {full} \
         ELSE CAST({unproven} AS VARCHAR) END)",
        within(FLOAT_TEXT_SHORT_RANGES),
        text(FLOAT_TEXT_SHORT_DIGITS),
        within(FLOAT_TEXT_LONG_RANGES),
    );
    bound(value, sql, &body)
}
