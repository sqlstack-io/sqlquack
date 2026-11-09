//! `json_guarded`: JSON text as a VARIANT, raising where DuckDB might read it differently.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::guards::{
    bound, fidelity_error, keys_repeated, long_decimal_pattern, unreadable, zero_fractions_whole,
};
use crate::semi_structured::constants::{
    JSON_GUARDED, JSON_NUMBER_PATTERN, JSON_STRING_PATTERN, JSON_SURROGATE_PATTERN,
    JSON_UNSAFE_TOKEN_PATTERN, JSON_WORD_PATTERN,
};

/// Text as DuckDB `JSON`, blank text NULL; text read differently raises, `trying` unfinished NULL.
pub fn json_guarded(sql: &str, trying: bool, log: &ProvenanceLog) -> String {
    log.building_block(JSON_GUARDED);
    let text = format!("CAST({sql} AS VARCHAR)");
    let tokens = outside_strings("r['t']");
    let written_numbers = format!(
        "regexp_extract_all({}, '{JSON_NUMBER_PATTERN}')",
        outside_strings("CAST(json(r['t']) AS VARCHAR)")
    );
    let invalid = fidelity_error("text DuckDB does not read as JSON, which Snowflake may read");
    let duplicates =
        keys_repeated("(CASE WHEN json_valid(json_source) THEN json(json_source) ELSE '[]' END)");
    let body = format!(
        "(CASE WHEN r['t'] IS NULL THEN NULL WHEN regexp_full_match(r['t'], ' *') THEN NULL \
         WHEN NOT r['valid'] THEN {invalid} \
         WHEN regexp_matches({tokens}, '{unsafe}') THEN {numbers} \
         WHEN regexp_matches({tokens}, '{long}') \
         AND regexp_extract_all({tokens}, '{JSON_NUMBER_PATTERN}') <> {written_numbers} \
         THEN {rewritten} \
         WHEN regexp_matches(r['t'], '{surrogate}') THEN {surrogates} \
         WHEN r['repeated'] THEN {repeated} \
         ELSE json(r['t']) END)",
        invalid = if trying {
            format!("(CASE WHEN {} THEN NULL ELSE {invalid} END)", unreadable("json_source"))
        } else {
            invalid
        },
        unsafe = quoted(JSON_UNSAFE_TOKEN_PATTERN),
        long = long_decimal_pattern(),
        rewritten = fidelity_error(
            "JSON with a long decimal DuckDB's double does not write back as written"
        ),
        numbers = fidelity_error(
            "JSON with a number, word or comma DuckDB may read differently from Snowflake"
        ),
        surrogate = quoted(JSON_SURROGATE_PATTERN),
        surrogates = fidelity_error("JSON with a UTF-16 surrogate escape"),
        repeated = fidelity_error("JSON with a key repeated in one object"),
    );
    bound(
        &text,
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

/// The JSON text `sql` with strings and the words true, false and null made empty strings.
fn outside_strings(sql: &str) -> String {
    format!(
        "regexp_replace(regexp_replace({sql}, '{}', '\"\"', 'g'), '{}', '\"\"', 'g')",
        quoted(JSON_STRING_PATTERN),
        quoted(JSON_WORD_PATTERN)
    )
}

fn quoted(pattern: &str) -> String {
    pattern.replace('\'', "''")
}
