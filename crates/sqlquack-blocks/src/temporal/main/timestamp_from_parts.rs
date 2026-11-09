//! `timestamp_from_parts`: a nanosecond timestamp from a date and nanoseconds into it.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::TIMESTAMP_RANGE;
use crate::raises::main::raised::raised;
use crate::temporal::constants::{NANOSECOND_DATE_RANGE, TIMESTAMP_FROM_PARTS};

/// The TIMESTAMP_NS `nanos` nanoseconds after midnight of `date`; a date beyond DuckDB's range raises.
pub fn timestamp_from_parts(date: &str, nanos: &str, log: &ProvenanceLog) -> String {
    log.building_block(TIMESTAMP_FROM_PARTS);
    let (first, last) = NANOSECOND_DATE_RANGE;
    let beyond = raised(&format!(
        "{TIMESTAMP_RANGE}: TIMESTAMP_NTZ_FROM_PARTS of a date beyond DuckDB's nanosecond range"
    ));
    format!(
        "list_transform([CAST({date} AS DATE)], lambda from_parts_date: CASE \
         WHEN from_parts_date NOT BETWEEN DATE '{first}' AND DATE '{last}' \
         THEN CAST({beyond} AS TIMESTAMP_NS) \
         ELSE make_timestamp_ns(epoch_ns(CAST(from_parts_date AS TIMESTAMP_NS)) + {nanos}) END)[1]"
    )
}
