//! The read check's bodies: the full check, and the fast check falling back to it.

use crate::macros::_helpers::guards::{
    bound, keys_repeated, long_decimal_pattern, quoted, site_fidelity_error, unreadable,
    zero_fractions_whole,
};
use crate::macros::constants::{
    JSON_READ_V1, OBJECTS_PARAMETER, SITE_PARAMETER, TEXT_PARAMETER, TRYING_PARAMETER,
    VALUES_PARAMETER,
};
use crate::macros::main::helper_call::helper_call;
use crate::semi_structured::constants::{
    JSON_NUMBER_PATTERN, JSON_STRING_PATTERN, JSON_SURROGATE_PATTERN, JSON_UNSAFE_TOKEN_PATTERN,
    JSON_WORD_PATTERN,
};

/// JSON whitespace.
const WHITESPACE: &str = r"[ \t\n\r]*";
/// A string without a raw control character or a UTF-16 surrogate escape.
const PLAIN_STRING: &str = r#""(?:[^"\\\x00-\x1f]|\\["/bfnrt]|\\u(?:[0-9a-cA-CefEF][0-9a-fA-F]{3}|[dD][0-7][0-9a-fA-F]{2}))*""#;
/// An integer of up to 18 digits but -0, or a decimal of up to 9 + 5 digits not all zero after the point.
const PLAIN_NUMBER: &str = r"-?[1-9][0-9]{0,17}|0|-?(?:0|[1-9][0-9]{0,8})\.(?:[0-9]{0,4}[1-9]|[0-9]{0,3}[1-9]0|[0-9]{0,2}[1-9]00|[0-9]?[1-9]000|[1-9]0000)";

/// The full check: the text as DuckDB `JSON`, blank text NULL, raising where Snowflake may differ.
pub(crate) fn read_body() -> String {
    let tokens = outside_strings("r['t']");
    let written_numbers = format!(
        "regexp_extract_all({}, '{JSON_NUMBER_PATTERN}')",
        outside_strings("CAST(json(r['t']) AS VARCHAR)")
    );
    let duplicates =
        keys_repeated("(CASE WHEN json_valid(json_source) THEN json(json_source) ELSE '[]' END)");
    let body = format!(
        "(CASE WHEN r['t'] IS NULL THEN NULL WHEN regexp_full_match(r['t'], ' *') THEN NULL \
         WHEN NOT r['valid'] THEN (CASE WHEN {TRYING_PARAMETER} AND {unreadable} THEN NULL \
         ELSE {invalid} END) \
         WHEN regexp_matches({tokens}, '{unsafe}') THEN {numbers} \
         WHEN regexp_matches({tokens}, '{long}') \
         AND regexp_extract_all({tokens}, '{JSON_NUMBER_PATTERN}') <> {written_numbers} \
         THEN {rewritten} \
         WHEN regexp_matches(r['t'], '{surrogate}') THEN {surrogates} \
         WHEN r['repeated'] THEN {repeated} \
         ELSE json(r['t']) END)",
        unreadable = unreadable("json_source"),
        invalid = site_fidelity_error("text DuckDB does not read as JSON, which Snowflake may read"),
        unsafe = quoted(JSON_UNSAFE_TOKEN_PATTERN),
        long = long_decimal_pattern(),
        rewritten = site_fidelity_error("JSON with a long decimal DuckDB's double does not write back as written"),
        numbers =
            site_fidelity_error("JSON with a number, word or comma DuckDB may read differently from Snowflake"),
        surrogate = quoted(JSON_SURROGATE_PATTERN),
        surrogates = site_fidelity_error("JSON with a UTF-16 surrogate escape"),
        repeated = site_fidelity_error("JSON with a key repeated in one object"),
    );
    bound(
        TEXT_PARAMETER,
        "json_source",
        &bound(
            &format!(
                "struct_pack(t := {}, valid := json_valid(json_source), \
                 repeated := {duplicates})",
                zero_fractions_whole("json_source")
            ),
            "r",
            &body,
        ),
    )
}

/// The fast check, binding the text as a lambda parameter so DuckDB never folds `json` of it.
pub(crate) fn checked_body() -> String {
    let t = "json_text";
    let unique_keys = format!(
        "coalesce(len(json_keys({t}, quack_path)) = \
         len(list_distinct(json_keys({t}, quack_path))), true)"
    );
    let unique_within = format!(
        "(coalesce(json_type({t}, quack_path) NOT IN ('OBJECT', 'ARRAY'), true) OR NOT {})",
        keys_repeated(&format!("json_extract({t}, quack_path)"))
    );
    bound(
        TEXT_PARAMETER,
        t,
        &format!(
            "(CASE WHEN regexp_full_match({t}, '{pattern}') AND json_valid({t}) \
             AND len(list_filter({OBJECTS_PARAMETER}, lambda quack_path: ({unique_keys}) IS NOT TRUE)) = 0 \
             AND len(list_filter({VALUES_PARAMETER}, lambda quack_path: {unique_within} IS NOT TRUE)) = 0 \
             THEN json({t}) ELSE {full} END)",
            pattern = quoted(&cleared_pattern()),
            full = helper_call(JSON_READ_V1, &[SITE_PARAMETER, t, TRYING_PARAMETER]),
        ),
    )
}

/// The text the fast check clears: plain strings, numbers and words between JSON punctuation.
fn cleared_pattern() -> String {
    let w = WHITESPACE;
    let item = format!(
        r"(?:[\[{{]{w})*(?:{PLAIN_STRING}|{PLAIN_NUMBER}|true|false|null|[\[{{]{w}[\]}}])(?:{w}[\]}}])*"
    );
    format!("{w}{item}(?:{w}[,:]{w}{item})*{w}")
}

/// The JSON text `sql` with strings and the words true, false and null made empty strings.
fn outside_strings(sql: &str) -> String {
    format!(
        "regexp_replace(regexp_replace({sql}, '{}', '\"\"', 'g'), '{}', '\"\"', 'g')",
        quoted(JSON_STRING_PATTERN),
        quoted(JSON_WORD_PATTERN)
    )
}
