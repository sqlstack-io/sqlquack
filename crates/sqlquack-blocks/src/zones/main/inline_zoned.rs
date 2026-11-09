//! `inline_zoned`: `text_to_zoned` written out, for its helper macros' bodies and VARIANT reads.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::models::TextFailure;
use crate::temporal::main::text_to_timestamp::text_to_timestamp;
use crate::zones::_helpers::offsets::zoned_instant;

/// `text_to_zoned` written out, as a helper macro's body and a VARIANT's string read it.
pub fn inline_zoned(value: &str, on_failure: TextFailure, log: &ProvenanceLog) -> String {
    let wall = text_to_timestamp("z", (on_failure, false), log);
    format!(
        "list_transform([{value}], lambda z: {})[1]",
        zoned_instant("z", &wall)
    )
}
