//! `text_to_zoned`: text in the recorded spellings as the UTC wall time of the instant it names.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::TEXT_TO_ZONED;
use crate::blocks::models::TextFailure;
use crate::macros::constants::{TRY_ZONED_TEXT_V1, ZONED_TEXT_V1};
use crate::macros::main::helper_call::helper_call;
use crate::raises::constants::UNIDENTIFIED_SITE;
use crate::zones::main::inline_zoned::inline_zoned;

/// A nanosecond UTC wall time: the text's wall time less its offset; text without one is UTC.
pub fn text_to_zoned(value: &str, on_failure: TextFailure, log: &ProvenanceLog) -> String {
    log.building_block(TEXT_TO_ZONED);
    let inline = inline_zoned(value, on_failure, log);
    match on_failure {
        TextFailure::Text => helper_call(ZONED_TEXT_V1, &[UNIDENTIFIED_SITE, value]),
        TextFailure::Null => helper_call(TRY_ZONED_TEXT_V1, &[UNIDENTIFIED_SITE, value]),
        TextFailure::Variant => inline,
    }
}
