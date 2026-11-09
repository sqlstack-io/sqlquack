//! `date_from_parts`: a DATE from year, month and day numbers, normalising overflow.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::temporal::constants::DATE_FROM_PARTS;

/// January 1 of the year moved by the months and days past the first, so overflow carries.
pub fn date_from_parts((year, month, day): (&str, &str, &str), log: &ProvenanceLog) -> String {
    log.building_block(DATE_FROM_PARTS);
    format!("CAST(make_date({year}, 1, 1) + to_months({month} - 1) + to_days({day} - 1) AS DATE)")
}
