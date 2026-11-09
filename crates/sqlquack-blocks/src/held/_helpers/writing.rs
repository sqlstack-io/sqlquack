//! JSON text of VARIANT-held values as Snowflake writes it.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::held::_helpers::guards::bound;
use crate::held::constants::{HELD_DOUBLE_FORMAT, HELD_DOUBLE_TOKEN_PATTERN, HELD_TOKEN_PATTERN};
use crate::semi_structured::main::json_escaped::json_escaped;

/// The JSON `sql` of a VARIANT-held value as Snowflake writes it: keys sorted, doubles as `%.15e`.
pub(crate) fn held_written(sql: &str, log: &ProvenanceLog) -> String {
    let token = HELD_TOKEN_PATTERN.replace('\'', "''");
    let respelled = format!(
        "(CASE WHEN left(held_tokens[i], 1) = '\"' THEN held_tokens[i] \
         WHEN regexp_matches(held_tokens[i], '{HELD_DOUBLE_TOKEN_PATTERN}') \
         THEN printf('{HELD_DOUBLE_FORMAT}', CAST(held_tokens[i] AS DOUBLE)) \
         ELSE held_tokens[i] END)"
    );
    let rebuilt = bound(
        &format!("regexp_extract_all(held_text, '{token}')"),
        "held_tokens",
        &format!(
            "array_to_string(list_transform(regexp_split_to_array(held_text, '{token}'), \
             lambda held_part, i: held_part || coalesce({respelled}, '')), '')"
        ),
    );
    bound(
        &format!("CAST({sql} AS VARCHAR)"),
        "held_text",
        &format!(
            "(CASE WHEN held_text IS NULL THEN NULL ELSE {} END)",
            json_escaped(&rebuilt, log)
        ),
    )
}
