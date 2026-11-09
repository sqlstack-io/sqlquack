//! `inline_utc_zoned`: `text_to_utc_zoned` written out, for its helper macros' bodies and VARIANT reads.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::TIMESTAMP_TZ_OFFSET;
use crate::blocks::models::TextFailure;
use crate::raises::main::raised::raised;
use crate::temporal::main::text_to_timestamp::text_to_timestamp;
use crate::zones::main::text_to_zoned::text_to_zoned;

/// `text_to_utc_zoned` written out, as a helper macro's body and a VARIANT's string read it.
pub fn inline_utc_zoned(value: &str, on_failure: TextFailure, log: &ProvenanceLog) -> String {
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
