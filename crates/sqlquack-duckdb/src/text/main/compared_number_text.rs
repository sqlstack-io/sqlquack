//! `compared_number_text`: text compared with a NUMBER, read as a NUMBER at that scale.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::constants::MAX_PRECISION;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::blocks::constants::{CONVERSION_WHITESPACE, INLINE_ARGUMENT_LIMIT};
use crate::blocks::main::text_to_number::text_to_number;
use crate::blocks::models::TextFailure;
use crate::raises::main::raised::raised;
use crate::text::constants::{COMPARED_NUMBER_TEXT, TEXT_NUMBER_SCALE};

/// The text `sql` as NUMBER(38, `scale`), as Snowflake reads it for a comparison, or the raise.
pub fn compared_number_text(sql: &str, scale: u8, log: &ProvenanceLog) -> Result<String, String> {
    log.building_block(COMPARED_NUMBER_TEXT);
    let number = SnowflakeType::Number {
        precision: MAX_PRECISION,
        scale,
    };
    let (bound, sql) = match sql.len() > INLINE_ARGUMENT_LIMIT {
        true => (Some(sql), "compared_text"),
        false => (None, sql),
    };
    let read = text_to_number(sql, number, TextFailure::Text, log)?;
    let compared = format!(
        "(CASE WHEN regexp_full_match(trim({sql}, {CONVERSION_WHITESPACE}), \
         '[^eE]*[.][0-9]{{{scale}}}0*[1-9][0-9]*|.*[0-9].*[eE].*') THEN CAST({} AS DECIMAL(38, {scale})) \
         ELSE {read} END)",
        raised(&format!(
            "{TEXT_NUMBER_SCALE}: text compared with a NUMBER that it spells beyond the NUMBER's scale"
        ))
    );
    Ok(match bound {
        Some(argument) => format!("list_transform([{argument}], lambda {sql}: {compared})[1]"),
        None => compared,
    })
}
