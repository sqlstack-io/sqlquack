//! LENGTH, LEFT, RIGHT, SUBSTR, LPAD, RPAD, POSITION and concatenation of BINARY, byte by byte.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::null_propagating_concat::null_propagating_concat;
use sqlquack_duckdb::raises::main::raised::raised;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::strings::emitted_text_operand;
use crate::conversion::_helpers::expressions::text_functions::integer_operand;
use crate::conversion::_helpers::support::scope::Emitted;
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::constants::{
    BYTE_CONCAT, BYTE_LENGTH, BYTE_PAD, BYTE_SEARCH, BYTE_SLICE, LENGTH_TYPE, TEXT_OCTET_LENGTH,
    TEXT_POSITION_LIMIT, TEXT_SEARCH_POSITION_TYPE, UNRECORDED_BYTES,
};

/// The DuckDB type BINARY is held in.
const BLOB: &str = "BLOB";

/// LENGTH, LEN or OCTET_LENGTH of BINARY: its count of bytes, typed NUMBER(18,0).
pub(crate) fn byte_length(bytes: &Emitted, site: Site<'_>) -> Emitted {
    let _rule = site.context.log.rule(BYTE_LENGTH);
    Emitted::new(
        format!("CAST(octet_length({}) AS DECIMAL(18,0))", bytes.sql),
        LENGTH_TYPE,
    )
}

/// OCTET_LENGTH: the count of bytes of BINARY, or of VARCHAR's UTF-8 bytes.
pub(crate) fn octet_length(
    value: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let value = convert_expression(value, site)?;
    if value.data_type == SnowflakeType::Binary {
        return Ok(byte_length(&value, site));
    }
    let _rule = site.context.log.rule(TEXT_OCTET_LENGTH);
    let text = emitted_text_operand(value, construct, site)?;
    Ok(Emitted::new(
        format!("CAST(octet_length(encode({text})) AS DECIMAL(18,0))"),
        LENGTH_TYPE,
    ))
}

/// LEFT or RIGHT of BINARY: its first or last `length` bytes; a negative length raises.
pub(crate) fn byte_left_right(
    name: &str,
    (bytes, length): (&Emitted, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(BYTE_SLICE);
    let length = integer_operand(length, construct, site)?;
    Ok(Emitted::new(
        format!(
            "list_transform([struct_pack(b := {}, n := {length})], lambda p: CASE \
             WHEN p['n'] < 0 THEN {} \
             ELSE unhex({}(hex(p['b']), 2 * {})) END)[1]",
            bytes.sql,
            unrecorded("a negative length", BLOB),
            name.to_ascii_lowercase(),
            clamped("p['n']")
        ),
        SnowflakeType::Binary,
    ))
}

/// SUBSTR or SUBSTRING of BINARY from a byte position of 1 or more; other positions raise.
pub(crate) fn byte_substring(
    (bytes, start, length): (&Emitted, &Value, Option<&Value>),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(BYTE_SLICE);
    let start = integer_operand(start, construct, site)?;
    let (length, digits) = match length {
        Some(length) => (
            integer_operand(length, construct, site)?,
            format!(", 2 * {}", clamped("p['n']")),
        ),
        None => ("0".to_owned(), String::new()),
    };
    Ok(Emitted::new(
        format!(
            "list_transform([struct_pack(b := {}, s := {start}, n := {length})], lambda p: CASE \
             WHEN p['s'] < 1 THEN {} WHEN p['n'] < 0 THEN {} \
             ELSE unhex(substring(hex(p['b']), 2 * {} - 1{digits})) \
             END)[1]",
            bytes.sql,
            unrecorded("a start position below 1", BLOB),
            unrecorded("a negative length", BLOB),
            clamped("p['s']"),
        ),
        SnowflakeType::Binary,
    ))
}

/// LPAD or RPAD of BINARY with BINARY padding, repeated from its first byte, or cut to the length.
pub(crate) fn byte_pad(
    name: &str,
    (bytes, length, filler): (&Emitted, &Value, Option<&Emitted>),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(BYTE_PAD);
    let Some(filler) = filler.filter(|filler| is_bytes(filler)) else {
        return Err(outside_conditions(
            construct.clone(),
            format!("{name} of BINARY without BINARY padding, which no recording shows"),
        ));
    };
    let length = integer_operand(length, construct, site)?;
    Ok(Emitted::new(
        format!(
            "list_transform([struct_pack(b := {}, n := {length}, f := {})], lambda p: CASE \
             WHEN p['n'] < 0 THEN {} WHEN octet_length(p['f']) = 0 THEN {} \
             ELSE unhex({}(hex(p['b']), CAST(2 * {} AS INTEGER), hex(p['f']))) \
             END)[1]",
            bytes.sql,
            filler.sql,
            unrecorded("a negative length", BLOB),
            unrecorded("empty padding", BLOB),
            name.to_ascii_lowercase(),
            clamped("p['n']")
        ),
        SnowflakeType::Binary,
    ))
}

/// The first byte position of BINARY `needle` in BINARY `haystack`, 0 when absent.
pub(crate) fn byte_position(
    (needle, haystack): (&Emitted, &Emitted),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(BYTE_SEARCH);
    if !is_bytes(needle) || !is_bytes(haystack) {
        return Err(mixed(construct));
    }
    Ok(Emitted::new(
        format!(
            "list_transform([struct_pack(n := hex({}), h := hex({}))], lambda p: CASE \
             WHEN p['n'] IS NULL OR p['h'] IS NULL THEN NULL WHEN p['n'] = '' THEN {} \
             ELSE CAST(coalesce(list_position(list_transform(\
             range(0, (length(p['h']) - length(p['n'])) // 2 + 1), \
             lambda i: substring(p['h'], 2 * i + 1, length(p['n'])) = p['n']), true), 0) \
             AS DECIMAL(9,0)) END)[1]",
            needle.sql,
            haystack.sql,
            unrecorded("an empty search", "DECIMAL(9,0)"),
        ),
        TEXT_SEARCH_POSITION_TYPE,
    ))
}

/// CONCAT, `||` or CONCAT_WS of BINARY parts: their bytes in order, NULL when any part is.
pub(crate) fn byte_concat(
    parts: &[&Emitted],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(BYTE_CONCAT);
    if !parts.iter().all(|part| is_bytes(part)) {
        return Err(mixed(construct));
    }
    let parts: Vec<String> = parts
        .iter()
        .map(|part| format!("CAST({} AS BLOB)", part.sql))
        .collect();
    Ok(Emitted::new(
        null_propagating_concat(&parts, site.context.log),
        SnowflakeType::Binary,
    ))
}

/// Whether `emitted` holds BINARY, or is an untyped NULL.
pub(crate) fn is_bytes(emitted: &Emitted) -> bool {
    matches!(
        emitted.data_type,
        SnowflakeType::Binary | SnowflakeType::Null
    )
}

fn mixed(construct: &Construct) -> Refusal {
    outside_conditions(
        construct.clone(),
        "BINARY beside another type, which needs an implicit conversion no recording shows"
            .to_owned(),
    )
}

/// An error raised for BINARY values the recordings do not reach, rather than a guessed result.
fn unrecorded(what: &str, held: &str) -> String {
    format!(
        "CAST({} AS {held})",
        raised(&format!("{UNRECORDED_BYTES}: {what}"))
    )
}

/// A byte count or position cut to the largest Snowflake reads, NULL staying NULL.
fn clamped(count: &str) -> String {
    format!(
        "(CASE WHEN {count} > {TEXT_POSITION_LIMIT} THEN {TEXT_POSITION_LIMIT} ELSE {count} END)"
    )
}
