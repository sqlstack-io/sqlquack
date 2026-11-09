//! `natural_logarithm`: LN as a double, failing for zero or less as Snowflake does.

use sqlquack_diagnostics::failures::constants::INVALID_FLOAT_OPERATION;
use sqlquack_diagnostics::failures::main::tagged_message::tagged_message;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::floats::constants::NATURAL_LOGARITHM;
use crate::raises::main::raised::raised;

/// LN of a value as a double; zero or less raises 100043.
pub fn natural_logarithm(value: &str, log: &ProvenanceLog) -> String {
    log.building_block(NATURAL_LOGARITHM);
    let invalid = raised(&tagged_message(
        INVALID_FLOAT_OPERATION,
        "Invalid floating point operation",
    ));
    format!(
        "list_transform([CAST({value} AS DOUBLE)], lambda x: CASE WHEN x <= 0 \
         THEN CAST({invalid} AS DOUBLE) ELSE ln(x) END)[1]"
    )
}
