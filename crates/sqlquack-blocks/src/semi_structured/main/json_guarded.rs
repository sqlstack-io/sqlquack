//! `json_guarded`: JSON text as a VARIANT, raising where DuckDB might read it differently.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::macros::constants::JSON_CHECKED_V1;
use crate::macros::main::helper_call::helper_call;
use crate::raises::constants::UNIDENTIFIED_SITE;
use crate::semi_structured::_helpers::paths::{path_list, plain_step, scoped_paths};
use crate::semi_structured::_helpers::plain_json::plain_json;
use crate::semi_structured::constants::JSON_GUARDED;
use crate::semi_structured::models::PathStep;

/// Text as DuckDB `JSON`, raising where read differently; only `reads`' paths are checked for repeats.
///
/// A literal the fast check would read as it is becomes `json` of the literal.
pub fn json_guarded(
    sql: &str,
    trying: bool,
    reads: Option<&[Vec<PathStep>]>,
    log: &ProvenanceLog,
) -> String {
    log.building_block(JSON_GUARDED);
    if literal_text(sql).is_some_and(|text| plain_json(&text)) {
        return format!("json({sql})");
    }
    let (objects, values) = match reads {
        Some(paths) if paths.iter().flatten().all(plain_step) => scoped_paths(paths),
        _ => (Vec::new(), vec!["$".to_owned()]),
    };
    helper_call(
        JSON_CHECKED_V1,
        &[
            UNIDENTIFIED_SITE,
            &format!("CAST({sql} AS VARCHAR)"),
            if trying { "true" } else { "false" },
            &path_list(&objects),
            &path_list(&values),
        ],
    )
}

/// The text of the single-quoted literal `sql`, its doubled quotes undone; `None` for other SQL.
fn literal_text(sql: &str) -> Option<String> {
    let inner = sql.strip_prefix('\'')?.strip_suffix('\'')?;
    let mut text = String::with_capacity(inner.len());
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character == '\'' && characters.next() != Some('\'') {
            return None;
        }
        text.push(character);
    }
    Some(text)
}
