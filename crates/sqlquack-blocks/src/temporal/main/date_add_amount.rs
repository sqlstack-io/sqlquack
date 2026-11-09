//! `date_add_amount`: a DATEADD amount, raising at magnitudes Snowflake rejects.

use sqlquack_diagnostics::failures::constants::NUMBER_OUT_OF_RANGE;
use sqlquack_diagnostics::failures::main::tagged_message::tagged_message;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::{DATE_ADD_AMOUNT, DATE_ADD_AMOUNT_LIMIT, NUMBER_RANGE_MESSAGE};
use crate::raises::main::raised::raised;

/// The whole NUMBER amount `sql`, failing as error 100046 beyond 2^53, as recorded.
pub fn date_add_amount(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(DATE_ADD_AMOUNT);
    format!(
        "list_transform([{sql}], lambda date_add_amount: CASE WHEN abs(date_add_amount) > \
         {DATE_ADD_AMOUNT_LIMIT} THEN {} ELSE date_add_amount END)[1]",
        raised(&tagged_message(NUMBER_OUT_OF_RANGE, NUMBER_RANGE_MESSAGE))
    )
}
