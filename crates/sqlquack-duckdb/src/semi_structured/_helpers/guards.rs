//! Run-time checks that raise where DuckDB's JSON could differ from Snowflake's VARIANT.

use crate::raises::main::raised::raised;
use crate::semi_structured::constants::{JSON_DECIMAL_DIGITS, JSON_FIDELITY};

/// An expression raising the json_fidelity error, naming `what`.
pub(crate) fn fidelity_error(what: &str) -> String {
    raised(&format!("{JSON_FIDELITY}: {what}"))
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
