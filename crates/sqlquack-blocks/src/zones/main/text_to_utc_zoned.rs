//! `text_to_utc_zoned`: text in the recorded spellings read as an instant whose offset is +00:00.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::TEXT_TO_UTC_ZONED;
use crate::blocks::models::TextFailure;
use crate::macros::constants::{TRY_UTC_ZONED_TEXT_V1, UTC_ZONED_TEXT_V1};
use crate::macros::main::helper_call::helper_call;
use crate::raises::constants::UNIDENTIFIED_SITE;
use crate::zones::main::inline_utc_zoned::inline_utc_zoned;

/// The instant `value` names, raising where its offset is not zero, so its wall time is UTC's.
pub fn text_to_utc_zoned(value: &str, on_failure: TextFailure, log: &ProvenanceLog) -> String {
    log.building_block(TEXT_TO_UTC_ZONED);
    let inline = inline_utc_zoned(value, on_failure, log);
    match on_failure {
        TextFailure::Text => helper_call(UTC_ZONED_TEXT_V1, &[UNIDENTIFIED_SITE, value]),
        TextFailure::Null => helper_call(TRY_UTC_ZONED_TEXT_V1, &[UNIDENTIFIED_SITE, value]),
        TextFailure::Variant => inline,
    }
}
