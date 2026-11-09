//! `inline_utc_zoned`: `text_to_utc_zoned` written out, for its helper macros' bodies and VARIANT reads.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::{TEXT_TO_ZONED, TIMESTAMP_TZ_OFFSET};
use crate::blocks::models::TextFailure;
use crate::raises::main::raised::raised;
use crate::temporal::main::text_to_timestamp::text_to_timestamp;
use crate::zones::_helpers::offsets::zoned_instant;

/// `text_to_utc_zoned` written out: the wall time read once, its instant derived as text_to_zoned does.
pub fn inline_utc_zoned(value: &str, on_failure: TextFailure, log: &ProvenanceLog) -> String {
    log.building_block(TEXT_TO_ZONED);
    let wall = text_to_timestamp("u", (on_failure, false), log);
    let instant = zoned_instant("u", "utc_wall");
    let offset = raised(&format!(
        "{TIMESTAMP_TZ_OFFSET}: TIMESTAMP_TZ text with an offset other than +00:00, whose wall \
         time DuckDB does not keep"
    ));
    format!(
        "list_transform([{value}], lambda u: list_transform([{wall}], lambda utc_wall: \
         list_transform([{instant}], lambda utc_instant: (CASE WHEN epoch_ns(utc_instant) \
         <> epoch_ns(utc_wall) THEN CAST({offset} AS TIMESTAMP_NS) ELSE utc_instant END))[1])[1])[1]"
    )
}
