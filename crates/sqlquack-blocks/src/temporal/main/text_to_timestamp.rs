//! `text_to_timestamp`: text in the recorded spellings as a nanosecond timestamp or a date.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::TEXT_TO_TIMESTAMP;
use crate::blocks::models::TextFailure;
use crate::macros::constants::{
    DATE_TEXT_V1, TIMESTAMP_TEXT_V1, TRY_DATE_TEXT_V1, TRY_TIMESTAMP_TEXT_V1,
};
use crate::macros::main::helper_call::helper_call;
use crate::raises::constants::UNIDENTIFIED_SITE;
use crate::temporal::main::inline_timestamp::inline_timestamp;

/// The AUTO spellings recorded, or epoch integer text; others raise, or are NULL as TRY_ recorded.
pub fn text_to_timestamp(
    value: &str,
    (on_failure, date_only): (TextFailure, bool),
    log: &ProvenanceLog,
) -> String {
    log.building_block(TEXT_TO_TIMESTAMP);
    match timestamp_macro(on_failure, date_only) {
        Some(name) => helper_call(name, &[UNIDENTIFIED_SITE, value]),
        None => inline_timestamp(value, (on_failure, date_only)),
    }
}

/// The helper macro reading text as a timestamp or a date; a VARIANT's string is read inline.
fn timestamp_macro(on_failure: TextFailure, date_only: bool) -> Option<&'static str> {
    match (on_failure, date_only) {
        (TextFailure::Text, false) => Some(TIMESTAMP_TEXT_V1),
        (TextFailure::Null, false) => Some(TRY_TIMESTAMP_TEXT_V1),
        (TextFailure::Text, true) => Some(DATE_TEXT_V1),
        (TextFailure::Null, true) => Some(TRY_DATE_TEXT_V1),
        (TextFailure::Variant, _) => None,
    }
}
