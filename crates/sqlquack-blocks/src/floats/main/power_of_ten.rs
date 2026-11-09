//! `power_of_ten`: POWER(10, n) of a whole exponent, rounded as the decimal 1en reads.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::floats::_helpers::values::bound;
use crate::floats::constants::POWER_OF_TEN;

/// 10 to the whole `exponent` as the double nearest it, ties to even; inf or 0 far out of range.
pub fn power_of_ten(exponent: &str, log: &ProvenanceLog) -> String {
    log.building_block(POWER_OF_TEN);
    bound(
        exponent,
        "n",
        "(CASE WHEN n IS NULL THEN NULL WHEN abs(n) <= 400 \
         THEN CAST('1e' || CAST(CAST(n AS HUGEINT) AS VARCHAR) AS DOUBLE) \
         WHEN n > 0 THEN CAST('inf' AS DOUBLE) ELSE CAST(0 AS DOUBLE) END)",
    )
}
