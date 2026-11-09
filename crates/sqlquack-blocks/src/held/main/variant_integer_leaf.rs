//! `variant_integer_leaf`: a whole NUMBER as a leaf of a VARIANT-held value.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::held::_helpers::guards::{bound, fidelity_error};
use crate::held::constants::{HELD_INTEGER_MAX, HELD_INTEGER_MIN, VARIANT_INTEGER_LEAF};

/// The whole NUMBER `sql` as JSON; values DuckDB's VARIANT would hold as a double raise.
pub fn variant_integer_leaf(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(VARIANT_INTEGER_LEAF);
    bound(
        sql,
        "held_integer",
        &format!(
            "(CASE WHEN held_integer < {HELD_INTEGER_MIN} OR held_integer > {HELD_INTEGER_MAX} \
             THEN CAST({} AS JSON) ELSE CAST(CAST(held_integer AS VARCHAR) AS JSON) END)",
            fidelity_error(
                "an integer in a VARIANT beyond 64 bits, which DuckDB holds as a double"
            )
        ),
    )
}
