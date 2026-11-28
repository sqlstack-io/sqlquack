//! Encodings of strings and byte strings: base64, hex and URL escapes, as Java's encoders and decoders write and read them.

use sqlquack_diagnostics::failures::constants::ARGUMENT_REJECTED;

use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::functions::arrays::captured;
use crate::databricks::conversion::_helpers::functions::rewrites::Rewrite;
use crate::databricks::conversion::constants::{
    BASE64_ALPHABET, BASE64_LINE, BASE64_READING_ERROR, BASE64_UNIT_ERROR, BASE64_UNITS,
    BASE64_UNPADDED, CHARSET_ERROR, CHARSETS, HEX_TEXT, LAMBDA_ELEMENT, REPLACEMENT_CHARACTER,
    UNCODED_CHARSET_ERROR, URL_DIGIT_ERROR, URL_ESCAPE_ERROR, URL_JAVA_DIGITS,
    URL_MALFORMED_ESCAPE, UTF8_CHARSET, UTF8_MALFORMED, UTF8_SEQUENCE,
};
use crate::databricks::conversion::models::{DatabricksType, Emitted};

/// The rewrite of the encoding function `name`, `None` when none handles it.
pub(crate) fn encoding_rewrite(name: &str) -> Option<Rewrite> {
    let rewrite: Rewrite = match name {
        "BASE64" => base64,
        "UNBASE64" => unbase64,
        "UNHEX" => unhex,
        "URL_ENCODE" => url_encode,
        "URL_DECODE" => |values| url_decode(values, false),
        "TRY_URL_DECODE" => |values| url_decode(values, true),
        "ENCODE" => |values| charset_coded(values, true),
        "DECODE" => |values| charset_coded(values, false),
        _ => return None,
    };
    Some(rewrite)
}

/// `BASE64` of a string's UTF-8 bytes or of a byte string, in lines of 76 characters broken with CR LF.
fn base64(values: &[Emitted]) -> Option<Emitted> {
    let [value] = values else { return None };
    let bytes = match value.data_type {
        DatabricksType::String => format!("encode({})", value.sql),
        DatabricksType::Binary => value.sql.clone(),
        _ => return None,
    };
    if captured(&value.sql) {
        return None;
    }
    let encoded = format!("to_base64({bytes})");
    Some(typed(
        format!(
            "array_to_string(list_transform(range(0, length({encoded}), {BASE64_LINE}), \
             lambda {LAMBDA_ELEMENT}: substr({encoded}, {LAMBDA_ELEMENT} + 1, {BASE64_LINE})), chr(13) || chr(10))"
        ),
        DatabricksType::String,
    ))
}

/// `UNBASE64(s)` as Java's MIME decoder reads it: characters outside the alphabet are skipped, padding may be left off; other shapes raise.
fn unbase64(values: &[Emitted]) -> Option<Emitted> {
    let [text] = values else { return None };
    if text.data_type != DatabricksType::String {
        return None;
    }
    let kept = format!("regexp_replace({}, '{BASE64_ALPHABET}', '', 'g')", text.sql);
    Some(typed(
        format!(
            "(CASE WHEN {kept} IS NULL THEN NULL \
             WHEN regexp_full_match({kept}, '{BASE64_UNITS}') THEN from_base64({kept}) \
             WHEN regexp_full_match({kept}, '{BASE64_UNPADDED}') \
             THEN from_base64({kept} || repeat('=', (4 - length({kept}) % 4) % 4)) \
             WHEN regexp_matches({kept}, '=[^=]') OR (NOT contains({kept}, '=') AND length({kept}) % 4 = 1) \
             THEN CAST(error('{ARGUMENT_REJECTED}: {BASE64_UNIT_ERROR}') AS BLOB) \
             ELSE CAST(error('{BASE64_READING_ERROR}') AS BLOB) END)"
        ),
        DatabricksType::Binary,
    ))
}

/// `UNHEX(s)`: the bytes of hex digits, a leading 0 added to an odd count, NULL for any other character.
fn unhex(values: &[Emitted]) -> Option<Emitted> {
    let [text] = values else { return None };
    if text.data_type != DatabricksType::String {
        return None;
    }
    let s = &text.sql;
    Some(typed(
        format!(
            "(CASE WHEN regexp_full_match({s}, '{HEX_TEXT}') \
             THEN unhex(CASE WHEN length({s}) % 2 = 1 THEN '0' || {s} ELSE {s} END) END)"
        ),
        DatabricksType::Binary,
    ))
}

/// `URL_ENCODE(s)` as Java's URLEncoder: DuckDB's escapes, with a space as `+`, `*` kept and `~` escaped.
fn url_encode(values: &[Emitted]) -> Option<Emitted> {
    let [text] = values else { return None };
    if text.data_type != DatabricksType::String {
        return None;
    }
    Some(typed(
        format!(
            "replace(replace(replace(url_encode({}), '%20', '+'), '%2A', '*'), '~', '%7E')",
            text.sql
        ),
        DatabricksType::String,
    ))
}

/// `URL_DECODE(s)` as Java's URLDecoder: `+` is a space; a malformed escape raises, or with `try` gives NULL.
fn url_decode(values: &[Emitted], try_decode: bool) -> Option<Emitted> {
    let [text] = values else { return None };
    if text.data_type != DatabricksType::String {
        return None;
    }
    let s = &text.sql;
    let malformed = match try_decode {
        true => "NULL".to_owned(),
        false => format!("CAST(error('{ARGUMENT_REJECTED}: {URL_ESCAPE_ERROR}') AS VARCHAR)"),
    };
    Some(typed(
        format!(
            "(CASE WHEN regexp_matches({s}, '{URL_JAVA_DIGITS}') THEN CAST(error('{URL_DIGIT_ERROR}') AS VARCHAR) \
             WHEN regexp_matches({s}, '{URL_MALFORMED_ESCAPE}') THEN {malformed} \
             ELSE url_decode(replace({s}, '+', ' ')) END)"
        ),
        DatabricksType::String,
    ))
}

/// `ENCODE(s, charset)` with `encode`, else `DECODE(b, charset)`, in UTF-8: NULL before the charset is read, raising for any other charset.
fn charset_coded(values: &[Emitted], encode: bool) -> Option<Emitted> {
    let [value, charset] = values else {
        return None;
    };
    let sql = match (value.data_type, encode) {
        (DatabricksType::String, true) => format!("encode({})", value.sql),
        (DatabricksType::Binary, false) if !captured(&value.sql) => replaced_utf8(&value.sql),
        (DatabricksType::String, false) => value.sql.clone(),
        _ => return None,
    };
    if charset.data_type != DatabricksType::String {
        return None;
    }
    let (v, c) = (&value.sql, &charset.sql);
    let named: Vec<String> = CHARSETS.iter().map(|name| format!("'{name}'")).collect();
    let data_type = match encode {
        true => DatabricksType::Binary,
        false => DatabricksType::String,
    };
    Some(typed(
        format!(
            "(CASE WHEN {v} IS NULL OR {c} IS NULL THEN NULL \
             WHEN lower({c}) = '{UTF8_CHARSET}' THEN {sql} \
             WHEN lower({c}) IN ({}) THEN error('{UNCODED_CHARSET_ERROR}') \
             ELSE error('{ARGUMENT_REJECTED}: {CHARSET_ERROR}') END)",
            named.join(", ")
        ),
        data_type,
    ))
}

/// The text of `bytes` as Java decodes UTF-8: each sequence Java reads as malformed, or each other byte, as U+FFFD.
fn replaced_utf8(bytes: &str) -> String {
    let e = LAMBDA_ELEMENT;
    format!(
        "(CASE WHEN regexp_full_match(hex({bytes}), '(?:{UTF8_SEQUENCE})*') THEN decode({bytes}) \
         ELSE array_to_string(list_transform(regexp_extract_all(hex({bytes}), '{UTF8_SEQUENCE}|{UTF8_MALFORMED}|[0-9A-F]{{2}}'), \
         lambda {e}: CASE WHEN regexp_full_match({e}, '{UTF8_SEQUENCE}') THEN decode(from_hex({e})) \
         ELSE chr({REPLACEMENT_CHARACTER}) END), '') END)"
    )
}
