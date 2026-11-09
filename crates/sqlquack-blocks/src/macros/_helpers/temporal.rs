//! The temporal text bodies: text read as a timestamp, a date or an instant, as Snowflake reads it.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::TIME_ZONE_SPELLING;
use crate::blocks::models::TextFailure;
use crate::macros::_helpers::guards::bound;
use crate::macros::constants::TEXT_PARAMETER;
use crate::raises::main::raised::raised;
use crate::raises::main::sited::sited;
use crate::temporal::main::inline_timestamp::inline_timestamp;
use crate::zones::constants::RECORDED_TIME_ZONES;
use crate::zones::main::inline_utc_zoned::inline_utc_zoned;
use crate::zones::main::inline_zoned::inline_zoned;

/// The text argument read once as a timestamp, or a date when `date_only`.
pub(crate) fn timestamp_body(on_failure: TextFailure, date_only: bool) -> String {
    sited(&bound(
        TEXT_PARAMETER,
        "t",
        &inline_timestamp("t", (on_failure, date_only)),
    ))
}

/// The text argument read as the UTC wall time of the instant it names.
pub(crate) fn zoned_body(on_failure: TextFailure) -> String {
    sited(&inline_zoned(
        TEXT_PARAMETER,
        on_failure,
        &ProvenanceLog::default(),
    ))
}

/// The text argument read as an instant whose offset must be +00:00.
pub(crate) fn utc_zoned_body(on_failure: TextFailure) -> String {
    sited(&inline_utc_zoned(
        TEXT_PARAMETER,
        on_failure,
        &ProvenanceLog::default(),
    ))
}

/// The text argument as it is when NULL or a recorded time zone name; other names raise.
pub(crate) fn zone_name_body() -> String {
    let names = RECORDED_TIME_ZONES
        .iter()
        .map(|zone| format!("'{zone}'"))
        .collect::<Vec<String>>()
        .join(", ");
    let unrecorded = raised(&format!(
        "{TIME_ZONE_SPELLING}: a time zone name whose spelling no recording shows Snowflake \
         accepting"
    ));
    sited(&bound(
        TEXT_PARAMETER,
        "z",
        &format!(
            "CASE WHEN z IS NULL OR z IN ({names}) THEN z ELSE CAST({unrecorded} AS VARCHAR) END"
        ),
    ))
}
