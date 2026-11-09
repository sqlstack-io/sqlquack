//! `json_scalar`: a VARIANT cast to NUMBER, FLOAT, BOOLEAN, DATE or TIMESTAMP_NTZ.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::blocks::main::cast::cast;
use crate::blocks::main::text_to_boolean::text_to_boolean;
use crate::blocks::main::text_to_float::text_to_float;
use crate::blocks::main::text_to_number::text_to_number;
use crate::blocks::models::TextFailure;
use crate::semi_structured::_helpers::guards::{bound, fidelity_error};
use crate::semi_structured::constants::{JSON_SCALAR, NANOS_PER_SECOND};
use crate::temporal::main::text_to_timestamp::text_to_timestamp;

/// Strings read as text, numbers as text or epoch seconds, JSON null as NULL; others raise.
pub fn json_scalar(
    sql: &str,
    target: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<String, String> {
    log.building_block(JSON_SCALAR);
    let string = "json_extract_string(j, '$')".to_owned();
    let number = "CAST(j AS VARCHAR)".to_owned();
    let numeric = "('BIGINT', 'UBIGINT', 'DOUBLE')";
    let refused = |kind: &str| {
        cast(
            &fidelity_error(&format!("a VARIANT holding {kind} cast to {target}")),
            target,
            log,
        )
    };
    let (from_string, from_number, from_boolean) = match target {
        SnowflakeType::Number { .. } => (
            text_to_number(&string, target, TextFailure::Variant, log)?,
            text_to_number(&number, target, TextFailure::Variant, log)?,
            cast(
                &format!("(CASE WHEN {string} = 'true' THEN 1 ELSE 0 END)"),
                target,
                log,
            )?,
        ),
        SnowflakeType::Float => (
            text_to_float(&string, TextFailure::Variant, log),
            text_to_float(&number, TextFailure::Variant, log),
            refused("a boolean")?,
        ),
        SnowflakeType::Boolean => (
            text_to_boolean(&string, TextFailure::Variant, log),
            refused("a number")?,
            format!("CAST({string} AS BOOLEAN)"),
        ),
        SnowflakeType::Date => (
            text_to_timestamp(&string, (TextFailure::Variant, true), log),
            refused("a number")?,
            refused("a boolean")?,
        ),
        SnowflakeType::TimestampNtz { precision: 9 } => (
            text_to_timestamp(&string, (TextFailure::Variant, false), log),
            format!(
                "(CASE WHEN json_type(j) IN ('BIGINT', 'UBIGINT') THEN \
                 make_timestamp_ns(CAST({number} AS BIGINT) * {NANOS_PER_SECOND}) ELSE {} END)",
                refused("a number with a fraction")?
            ),
            refused("a boolean")?,
        ),
        _ => {
            return Err(format!(
                "casting a VARIANT to {target} has no proven conversion"
            ));
        }
    };
    let body = format!(
        "(CASE WHEN json_type(j) = 'NULL' THEN NULL \
         WHEN json_type(j) = 'VARCHAR' THEN {from_string} \
         WHEN json_type(j) IN {numeric} THEN {from_number} \
         WHEN json_type(j) = 'BOOLEAN' THEN {from_boolean} \
         WHEN j IS NULL THEN NULL ELSE {} END)",
        refused("an object or array")?
    );
    Ok(bound(sql, "j", &body))
}
