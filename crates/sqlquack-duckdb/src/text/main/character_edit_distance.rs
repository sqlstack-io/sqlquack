//! `character_edit_distance`: the Levenshtein distance between two texts, counted in characters.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::text::constants::CHARACTER_EDIT_DISTANCE;

/// Levenshtein distance over code points, binding each operand once; NULL when either is NULL.
pub fn character_edit_distance(left: &str, right: &str, log: &ProvenanceLog) -> String {
    log.building_block(CHARACTER_EDIT_DISTANCE);
    format!(
        "list_transform([struct_pack(l := {left}, r := {right})], lambda p: CASE \
         WHEN p['l'] IS NULL OR p['r'] IS NULL THEN NULL ELSE list_reduce(list_transform(\
         range(1, length(p['l']) + 1), lambda i: struct_pack(c := substring(p['l'], i, 1), i := i)), \
         lambda previous, x: list_reduce(range(1, length(p['r']) + 1), lambda row, j: \
         list_append(row, least(previous[j + 1] + 1, row[j] + 1, previous[j] + \
         CAST(x['c'] <> substring(p['r'], j, 1) AS BIGINT))), [x['i']]), \
         range(0, length(p['r']) + 1))[length(p['r']) + 1] END)[1]"
    )
}
