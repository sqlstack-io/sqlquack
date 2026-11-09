//! `text_to_utc_zoned`: text in the recorded spellings read as an instant whose offset is +00:00.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::{TEXT_TO_UTC_ZONED, TIMESTAMP_TZ_OFFSET};
use crate::blocks::models::TextFailure;
use crate::raises::main::raised::raised;
use crate::temporal::main::text_to_timestamp::text_to_timestamp;
use crate::temporal::main::text_to_zoned::text_to_zoned;

/// The instant `value` names, raising where its offset is not zero, so its wall time is UTC's.
pub fn text_to_utc_zoned(value: &str, on_failure: TextFailure, log: &ProvenanceLog) -> String {
    log.building_block(TEXT_TO_UTC_ZONED);
    let instant = text_to_zoned("u", on_failure, log);
    let wall = text_to_timestamp("u", (on_failure, false), log);
    let offset = raised(&format!(
        "{TIMESTAMP_TZ_OFFSET}: TIMESTAMP_TZ text with an offset other than +00:00, whose wall \
         time DuckDB does not keep"
    ));
    format!(
        "list_transform([{value}], lambda u: (CASE WHEN epoch_ns({instant}) <> epoch_ns({wall}) \
         THEN CAST({offset} AS TIMESTAMP_NS) ELSE {instant} END))[1]"
    )
}
