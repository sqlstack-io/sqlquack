//! Run-time checks the helper macros' bodies raise on, where DuckDB's JSON could differ from Snowflake's.

use crate::macros::constants::SITE_PARAMETER;
use crate::raises::main::raised::raised;
use crate::raises::main::site_raised::site_raised;
use crate::semi_structured::constants::{
    JSON_DECIMAL_DIGITS, JSON_FIDELITY, JSON_STRING_PATTERN, KEPT_NUMBER_PATTERN,
    ZERO_FRACTION_PATTERN,
};

/// An expression raising the json_fidelity error, naming `what`, with an unidentified site.
pub(crate) fn fidelity_error(what: &str) -> String {
    raised(&format!("{JSON_FIDELITY}: {what}"))
}

/// A json_fidelity raise naming `what`, whose ID the call's site argument holds.
pub(crate) fn site_fidelity_error(what: &str) -> String {
    site_raised(&format!("{JSON_FIDELITY}: {what}"), SITE_PARAMETER, None)
}

/// A pattern matching a decimal number of more than `JSON_DECIMAL_DIGITS` digits.
pub(crate) fn long_decimal_pattern() -> String {
    let limit = JSON_DECIMAL_DIGITS + 1;
    let mut alternatives: Vec<String> = (1..limit)
        .map(|whole| format!("[0-9]{{{whole}}}\\.[0-9]{{{}}}", limit - whole))
        .collect();
    alternatives.push(format!("[0-9]{{{limit}}}\\.[0-9]"));
    alternatives.join("|")
}

/// Whether some object in the JSON value `sql` lists its keys out of byte order.
pub(crate) fn keys_unsorted(sql: &str) -> String {
    any_object(sql, "json_keys(o) <> list_sort(json_keys(o))")
}

/// Whether some object in the JSON value `sql` repeats a key.
pub(crate) fn keys_repeated(sql: &str) -> String {
    any_object(sql, "len(json_keys(o)) <> len(list_distinct(json_keys(o)))")
}

/// Whether `test` holds for an object `o` at any depth of `sql`, without a slow subquery.
fn any_object(sql: &str, test: &str) -> String {
    bound(
        &format!("CAST({sql} AS JSON)"),
        "json_root",
        &format!(
            "coalesce(len(list_filter(list_prepend(json_root, json_extract(json_root, '$..*')), \
             lambda o: json_type(o) = 'OBJECT' AND {test})) > 0, false)"
        ),
    )
}

/// `body` over `value` evaluated once, as the lambda parameter `name`.
pub(crate) fn bound(value: &str, name: &str, body: &str) -> String {
    format!("list_transform([{value}], lambda {name}: {body})[1]")
}

/// JSON text with all-zero fractions outside strings dropped, as Snowflake reads them; not -0.0.
pub(crate) fn zero_fractions_whole(sql: &str) -> String {
    let pattern = format!("({JSON_STRING_PATTERN}|{KEPT_NUMBER_PATTERN}){ZERO_FRACTION_PATTERN}")
        .replace('\'', "''");
    format!("regexp_replace({sql}, '{pattern}', '\\1\\2\\3', 'g')")
}

/// Whether `sql` is markup, or a document cut off before it closes with no lenient quoting.
pub(crate) fn unreadable(sql: &str) -> String {
    let strings = JSON_STRING_PATTERN.replace('\'', "''");
    let outside = format!("regexp_replace({sql}, '(?s){strings}', '', 'g')");
    let before_open_string = bound(
        &outside,
        "unquoted",
        "(CASE WHEN strpos(unquoted, '\"') > 0 \
         THEN left(unquoted, strpos(unquoted, '\"') - 1) ELSE unquoted END)",
    );
    let unfinished = bound(
        &before_open_string,
        "head",
        "(regexp_full_match(head, '(?s)\\s*[\\[{][^''`/#\\\\]*') \
         AND list_reduce(list_transform(\
         string_split(regexp_replace(head, '[^\\[\\]{}]', '', 'g'), ''), \
         lambda c: CASE WHEN c IN ('[', '{') THEN 1 ELSE -1 END), \
         lambda depth, step: CASE WHEN depth <= 0 THEN -1 ELSE depth + step END) > 0)",
    );
    format!("(regexp_matches({sql}, '^\\s*<') OR {unfinished})")
}

/// A pattern quoted for a SQL string literal.
pub(crate) fn quoted(pattern: &str) -> String {
    pattern.replace('\'', "''")
}
