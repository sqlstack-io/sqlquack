//! FLATTEN's PATH text: each object key written as Snowflake writes it in a path.

use crate::semi_structured::_helpers::guards::fidelity_error;
use crate::semi_structured::constants::{BRACKETED_KEY, PLAIN_KEY_PATTERN};

/// The path step of the decoded key `key`: `.key` for a plain name, `['key']` with `'` escaped
/// for other printable ASCII, and a raise for any other key.
pub(crate) fn key_step(key: &str) -> String {
    format!(
        "CASE WHEN regexp_full_match({key}, '{PLAIN_KEY_PATTERN}') THEN '.' || {key} \
         WHEN regexp_full_match({key}, '{BRACKETED_KEY}') \
         THEN '[''' || replace({key}, '''', '\\''') || ''']' \
         ELSE CAST({} AS VARCHAR) END",
        fidelity_error("a FLATTEN path through a key holding a character no recording shows")
    )
}

/// The path `steps` spell, each step `.key` or `[index]`, without a leading dot.
pub(crate) fn joined_path(steps: &str) -> String {
    format!("regexp_replace(array_to_string({steps}, ''), '^\\.', '')")
}
