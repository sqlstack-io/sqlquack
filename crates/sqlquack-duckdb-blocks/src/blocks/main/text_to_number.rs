//! `text_to_number`: text to NUMBER, accepting only spellings Snowflake is recorded to read.

use sqlquack_core::failures::constants::NUMBER_NOT_RECOGNIZED;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::blocks::_helpers::numbers::{largest_number, read_number_as};
use crate::blocks::_helpers::text::{conversion_failure, once_bound, overflow_failure};
use crate::blocks::_helpers::types_map::duckdb_type;
use crate::blocks::constants::TEXT_TO_NUMBER;
use crate::blocks::models::{NumberTarget, NumberText, TextFailure};

/// Text to `target`, rounding half away from zero; unrecorded spellings and overflow fail.
pub fn text_to_number(
    sql: &str,
    target: NumberText,
    log: &ProvenanceLog,
) -> Result<String, String> {
    log.building_block(TEXT_TO_NUMBER);
    let (result, failure) = match target {
        NumberText::Typed(result, failure) => (result, failure),
        NumberText::Spelled(spelled) => return Ok(read_number_as(sql, &spelled)),
    };
    let SnowflakeType::Number { precision, scale } = result else {
        return Err(format!(
            "text_to_number needs a NUMBER target, not {result}"
        ));
    };
    let target = duckdb_type(result, log)?;
    Ok(once_bound(sql, "number_text", |sql| {
        read_number(sql, (&target, precision, scale), failure)
    }))
}

/// The CASE reading the text `sql` as the DuckDB `target` of NUMBER(`precision`, `scale`).
fn read_number(
    sql: &str,
    (target, precision, scale): (&str, u8, u8),
    failure: TextFailure,
) -> String {
    let largest = largest_number(precision, scale);
    read_number_as(
        sql,
        &NumberTarget {
            cast: ("CAST(".to_owned(), format!(" AS {target})")),
            try_cast: ("TRY_CAST(".to_owned(), format!(" AS {target})")),
            largest: format!("CAST('{largest}' AS {target})"),
            overflow: format!("CAST({} AS {target})", overflow_failure(failure)),
            unrecognised: format!(
                "CAST({} AS {target})",
                conversion_failure(
                    failure,
                    (NUMBER_NOT_RECOGNIZED, "Numeric value is not recognized")
                )
            ),
        },
    )
}
