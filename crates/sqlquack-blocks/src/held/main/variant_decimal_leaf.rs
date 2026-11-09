//! `variant_decimal_leaf`: a NUMBER with a scale as a leaf of a VARIANT-held value.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::held::_helpers::guards::{bound, fidelity_error};
use crate::held::constants::{HELD_INTEGER_MAX, HELD_INTEGER_MIN, VARIANT_DECIMAL_LEAF};

/// A whole value as the integer Snowflake writes; a fraction, which DuckDB would hold as a double, raises.
pub fn variant_decimal_leaf(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(VARIANT_DECIMAL_LEAF);
    bound(
        sql,
        "held_decimal",
        &format!(
            "(CASE WHEN held_decimal <> trunc(held_decimal) OR held_decimal < {HELD_INTEGER_MIN} \
             OR held_decimal > {HELD_INTEGER_MAX} THEN CAST({} AS JSON) \
             ELSE CAST(CAST(CAST(trunc(held_decimal) AS HUGEINT) AS VARCHAR) AS JSON) END)",
            fidelity_error(
                "a NUMBER with a fraction beside a FLOAT in one VARIANT, which DuckDB holds as a \
                 double"
            )
        ),
    )
}
