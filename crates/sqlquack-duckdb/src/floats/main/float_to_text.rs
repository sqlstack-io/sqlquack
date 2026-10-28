//! `float_to_text`: a FLOAT as the text Snowflake writes for it.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::floats::_helpers::values::bound;
use crate::floats::constants::{
    FLOAT_TEXT, FLOAT_TEXT_PLACES, FLOAT_TO_TEXT, SMALLEST_NORMAL_DOUBLE,
};
use crate::raises::main::raised::raised;

/// `%.15g` text, `NaN`, `inf` and `-inf`; text no recording pins down raises `float_text`.
pub fn float_to_text(value: &str, log: &ProvenanceLog) -> String {
    log.building_block(FLOAT_TO_TEXT);
    let sql = "f";
    let unproven = raised(&format!(
        "{FLOAT_TEXT}: a FLOAT whose text no recording shows: negative zero, subnormal or with \
         digits past the fifteenth decimal place"
    ));
    let text = format!("printf('%.15g', {sql})");
    let places = format!(
        "(CASE WHEN contains({text}, 'e') THEN length(regexp_extract({text}, '[.]([0-9]+)e', 1)) \
         + CAST(regexp_extract({text}, 'e-([0-9]+)$', 1) AS INTEGER) \
         ELSE length(regexp_extract({text}, '[.]([0-9]+)$', 1)) END)"
    );
    let body = format!(
        "(CASE WHEN {sql} IS NULL THEN NULL WHEN isnan({sql}) THEN 'NaN' \
         WHEN isinf({sql}) THEN CASE WHEN {sql} > 0 THEN 'inf' ELSE '-inf' END \
         WHEN {sql} = 0 AND NOT signbit({sql}) THEN '0' \
         WHEN abs({sql}) < {SMALLEST_NORMAL_DOUBLE} \
         OR abs({sql}) < 1 AND {places} > {FLOAT_TEXT_PLACES} THEN CAST({unproven} AS VARCHAR) \
         ELSE {text} END)"
    );
    bound(value, sql, &body)
}
