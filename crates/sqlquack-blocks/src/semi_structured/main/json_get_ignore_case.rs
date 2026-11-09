//! `json_get_ignore_case`: GET_IGNORE_CASE of an object and a literal ASCII key.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::main::string_literal::string_literal;
use crate::semi_structured::_helpers::guards::{bound, fidelity_error};
use crate::semi_structured::constants::JSON_GET_IGNORE_CASE;

/// The exact key, else the one ASCII-case match; NULL for none, raising for several or non-ASCII.
pub fn json_get_ignore_case(sql: &str, key: &str, log: &ProvenanceLog) -> Result<String, String> {
    log.building_block(JSON_GET_IGNORE_CASE);
    if !key.is_ascii() || key.contains(['"', '\\']) {
        return Err(format!(
            "GET_IGNORE_CASE of the key `{key}`, not plain ASCII"
        ));
    }
    let exact = string_literal(&format!("$.\"{key}\""), log);
    let lower = string_literal(&key.to_ascii_lowercase(), log);
    let folded = "translate(k, 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz')";
    let matches = format!("list_filter(json_keys(j), lambda k: {folded} = {lower})");
    let body = format!(
        "(CASE WHEN json_type(j) IS DISTINCT FROM 'OBJECT' THEN NULL \
         WHEN json_exists(j, {exact}) THEN json_extract(j, {exact}) \
         WHEN len(list_filter(json_keys(j), lambda k: NOT regexp_full_match(k, '[ -!#-\\[\\]-~]*'))) > 0 \
         THEN {plain} \
         WHEN len({matches}) > 1 THEN {several} \
         WHEN len({matches}) = 0 THEN NULL \
         ELSE json_extract(j, '$.\"' || {matches}[1] || '\"') END)",
        plain = fidelity_error("GET_IGNORE_CASE over a key that is not plain ASCII"),
        several = fidelity_error("GET_IGNORE_CASE matching several keys"),
    );
    Ok(bound(sql, "j", &body))
}
