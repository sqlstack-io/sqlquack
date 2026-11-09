//! The casting bodies: a VARIANT cast to NUMBER, FLOAT, BOOLEAN, DATE or TIMESTAMP_NTZ.

use sqlquack_diagnostics::failures::constants::{NUMBER_OUT_OF_RANGE, VARIANT_NOT_CAST};
use sqlquack_diagnostics::failures::main::tagged_message::tagged_message;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::blocks::constants::{NUMBER_RANGE_MESSAGE, VARIANT_CAST_MESSAGE};
use crate::blocks::main::cast::cast;
use crate::blocks::main::text_to_boolean::text_to_boolean;
use crate::blocks::main::text_to_float::text_to_float;
use crate::blocks::main::text_to_number::text_to_number;
use crate::blocks::models::{NumberTarget, NumberText, TextFailure};
use crate::macros::_helpers::guards::{bound, fidelity_error};
use crate::macros::constants::{
    NUMBER_LARGEST_PARAMETER, NUMBER_TYPE_PARAMETER, SITE_PARAMETER, VALUE_PARAMETER,
    VARIANT_BOOLEAN_V1, VARIANT_DATE_V1, VARIANT_FLOAT_V1, VARIANT_TIMESTAMP_NTZ_V1,
};
use crate::raises::main::site_raised::site_raised;
use crate::raises::main::sited::sited;
use crate::semi_structured::constants::{JSON_FIDELITY, NANOS_PER_SECOND};
use crate::temporal::main::text_to_timestamp::text_to_timestamp;

/// A VARIANT to the NUMBER type of the typed NULL the call passes: strings read as number text.
pub(crate) fn number_body() -> String {
    let typed = |sql: &str| format!("cast_to_type({sql}, {NUMBER_TYPE_PARAMETER})");
    let trying = |sql: &str| format!("TRY(cast_to_type({sql}, {NUMBER_TYPE_PARAMETER}))");
    let overflow = typed(&site_raised(
        &tagged_message(NUMBER_OUT_OF_RANGE, NUMBER_RANGE_MESSAGE),
        SITE_PARAMETER,
        None,
    ));
    let from_string = text_to_number(
        "json_extract_string(j, '$')",
        NumberText::Spelled(NumberTarget {
            cast: (
                "cast_to_type(".to_owned(),
                format!(", {NUMBER_TYPE_PARAMETER})"),
            ),
            try_cast: (
                "TRY(cast_to_type(".to_owned(),
                format!(", {NUMBER_TYPE_PARAMETER}))"),
            ),
            largest: NUMBER_LARGEST_PARAMETER.to_owned(),
            overflow: overflow.clone(),
            unrecognised: typed(&site_raised(
                &tagged_message(VARIANT_NOT_CAST, VARIANT_CAST_MESSAGE),
                SITE_PARAMETER,
                None,
            )),
        }),
        &ProvenanceLog::default(),
    )
    .unwrap_or_default();
    let objects = typed(&site_raised(
        &format!("{JSON_FIDELITY}: a VARIANT holding an object or array cast to "),
        SITE_PARAMETER,
        Some(&format!(
            "replace(typeof({NUMBER_TYPE_PARAMETER}), 'DECIMAL', 'NUMBER')"
        )),
    ));
    format!(
        "list_transform([{VALUE_PARAMETER}], lambda j: (CASE WHEN json_type(j) = 'NULL' \
         THEN NULL WHEN json_type(j) = 'VARCHAR' THEN {from_string} \
         WHEN json_type(j) IN ('BIGINT', 'UBIGINT', 'DOUBLE') THEN (CASE \
         WHEN abs({number}) <= {NUMBER_LARGEST_PARAMETER} THEN {number} ELSE {overflow} END) \
         WHEN json_type(j) = 'BOOLEAN' \
         THEN {} WHEN j IS NULL THEN NULL ELSE {objects} END))[1]",
        typed("(CASE WHEN json_extract_string(j, '$') = 'true' THEN 1 ELSE 0 END)"),
        number = trying("j"),
    )
}

/// A VARIANT cast to `target` other than NUMBER, raising at the call's site.
pub(crate) fn scalar_body(target: SnowflakeType) -> String {
    sited(&scalar_case(target, &ProvenanceLog::default()).unwrap_or_default())
}

/// The helper macro casting a VARIANT to `target`, other than NUMBER.
pub(crate) fn scalar_macro(target: SnowflakeType) -> Option<&'static str> {
    match target {
        SnowflakeType::Float => Some(VARIANT_FLOAT_V1),
        SnowflakeType::Boolean => Some(VARIANT_BOOLEAN_V1),
        SnowflakeType::Date => Some(VARIANT_DATE_V1),
        SnowflakeType::TimestampNtz { precision: 9 } => Some(VARIANT_TIMESTAMP_NTZ_V1),
        _ => None,
    }
}

/// The largest NUMBER(`precision`, `scale`) as text, such as `9999.99`.
pub(crate) fn largest_number(precision: u8, scale: u8) -> String {
    let whole = "9".repeat(usize::from(precision - scale));
    match scale {
        0 => whole,
        _ => format!("{whole}.{}", "9".repeat(usize::from(scale))),
    }
}

/// Strings read as text, numbers as text or epoch seconds, JSON null as NULL; others raise.
pub(crate) fn scalar_case(target: SnowflakeType, log: &ProvenanceLog) -> Result<String, String> {
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
    Ok(bound(VALUE_PARAMETER, "j", &body))
}
