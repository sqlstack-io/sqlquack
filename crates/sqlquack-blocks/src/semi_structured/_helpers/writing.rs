//! JSON text as Snowflake writes it, through the helper macros that write it.

use crate::macros::constants::{JSON_ESCAPED_V1, VARIANT_JSON_TEXT_V1, VARIANT_TEXT_V1};
use crate::macros::main::helper_call::helper_call;
use crate::raises::constants::UNIDENTIFIED_SITE;

/// What a VARIANT's root becomes when written.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Root {
    /// Every value as JSON text, JSON null as `null`: TO_JSON.
    Json,
    /// A string root unquoted and JSON null as SQL NULL; other values as JSON text: `::VARCHAR`.
    Text,
}

/// The VARIANT `sql` as Snowflake writes it, through its helper macro.
pub(crate) fn sorted_text(sql: &str, root: Root) -> String {
    let name = match root {
        Root::Json => VARIANT_JSON_TEXT_V1,
        Root::Text => VARIANT_TEXT_V1,
    };
    helper_call(name, &[UNIDENTIFIED_SITE, sql])
}

/// DuckDB's JSON text `sql` with Snowflake's escapes, through its helper macro.
pub(crate) fn escaped(sql: &str) -> String {
    helper_call(JSON_ESCAPED_V1, &[sql])
}
