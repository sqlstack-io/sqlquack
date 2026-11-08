//! `square_root`: SQRT as a double, failing for a negative number as Snowflake does.

use sqlquack_core::failures::constants::SQUARE_ROOT_OF_NEGATIVE;
use sqlquack_core::failures::main::tagged_message::tagged_message;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::floats::constants::SQUARE_ROOT;
use crate::raises::main::raised::raised;

/// SQRT of a value as a double; a negative number raises 100044.
pub fn square_root(value: &str, log: &ProvenanceLog) -> String {
    log.building_block(SQUARE_ROOT);
    let negative = raised(&tagged_message(
        SQUARE_ROOT_OF_NEGATIVE,
        "Invalid floating point operation",
    ));
    format!(
        "list_transform([CAST({value} AS DOUBLE)], lambda x: CASE WHEN x < 0 \
         THEN CAST({negative} AS DOUBLE) ELSE sqrt(x) END)[1]"
    )
}
