//! `compared_number_range`: a NUMBER compared with text, raising where Snowflake overflows.

use sqlquack_core::failures::constants::NUMBER_OUT_OF_RANGE;
use sqlquack_core::failures::main::tagged_message::tagged_message;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::constants::MAX_PRECISION;

use crate::blocks::constants::NUMBER_RANGE_MESSAGE;
use crate::raises::main::raised::raised;
use crate::text::constants::{
    COMPARED_NUMBER_RANGE, TEXT_COMPARED_SAFE_SCALE, TEXT_COMPARED_WHOLE_DIGITS, TEXT_NUMBER_SCALE,
};

/// The NUMBER(`precision`, `scale`) `sql`; 38 whole digits fail as 100046, unrecorded widths raise.
pub fn compared_number_range(
    sql: &str,
    (precision, scale): (u8, u8),
    log: &ProvenanceLog,
) -> String {
    if precision - scale <= TEXT_COMPARED_WHOLE_DIGITS || scale >= TEXT_COMPARED_SAFE_SCALE {
        return sql.to_owned();
    }
    let power = |digits: u8| {
        format!(
            "CAST('1{}' AS DECIMAL({precision}, {scale}))",
            "0".repeat(usize::from(digits))
        )
    };
    log.building_block(COMPARED_NUMBER_RANGE);
    let unrecorded = raised(&format!(
        "{TEXT_NUMBER_SCALE}: a NUMBER compared with text, wider than recorded comparisons"
    ));
    let overflow = match precision - scale == MAX_PRECISION {
        true => format!(
            "WHEN abs(compared_number) >= {} THEN CAST({} AS DECIMAL({precision}, {scale})) ",
            power(MAX_PRECISION - 1),
            raised(&tagged_message(NUMBER_OUT_OF_RANGE, NUMBER_RANGE_MESSAGE))
        ),
        false => String::new(),
    };
    format!(
        "list_transform([{sql}], lambda compared_number: CASE \
         WHEN compared_number IS NULL OR abs(compared_number) < {} THEN compared_number \
         {overflow}ELSE CAST({unrecorded} AS DECIMAL({precision}, {scale})) END)[1]",
        power(TEXT_COMPARED_WHOLE_DIGITS),
    )
}
