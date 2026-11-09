//! `json_array_sort`: ARRAY_SORT of one array, ascending with JSON nulls last.

use sqlquack_diagnostics::failures::constants::NOT_AN_ARRAY_ARGUMENT;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::arrays::_helpers::elements::{bound, elements, json_array, json_error};
use crate::arrays::constants::{
    ARRAY_SORT_ORDER, JSON_ARRAY_SORT, NOT_AN_ARRAY_MESSAGE, SORT_ARRAY_RANK, SORT_KIND_ORDER,
    SORT_OBJECT_RANK,
};
use crate::raises::main::raised::raised;

/// The array sorted: booleans, numbers by value, strings by bytes, objects, arrays, then JSON nulls.
pub fn json_array_sort(array: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_SORT);
    let mut ranks: Vec<String> = Vec::new();
    for (rank, kinds) in SORT_KIND_ORDER.iter().enumerate() {
        for kind in *kinds {
            ranks.push(format!("WHEN '{kind}' THEN {rank}"));
        }
    }
    let rank = format!("CASE json_type(e) {} END", ranks.join(" "));
    let unordered = raised(&format!(
        "{ARRAY_SORT_ORDER}: ARRAY_SORT of objects, arrays or numbers whose order no recording shows"
    ));
    let keyed = format!(
        "list_transform({}, lambda e: struct_pack(r := {rank}, \
         b := CASE WHEN json_type(e) = 'BOOLEAN' THEN CAST(e AS BOOLEAN) END, \
         n := CASE WHEN json_type(e) IN ('UBIGINT', 'BIGINT', 'DOUBLE') THEN CAST(e AS DOUBLE) END, \
         s := CASE WHEN json_type(e) = 'VARCHAR' THEN json_extract_string(e, '$') END, \
         t := CAST(e AS VARCHAR)))",
        elements("q")
    );
    let several = |rank: usize| {
        format!(
            "len(list_distinct(list_transform(list_filter(k, lambda x: x['r'] = {rank}), \
             lambda x: x['t']))) > 1"
        )
    };
    let ambiguous = format!(
        "{} OR {} OR len(",
        several(SORT_OBJECT_RANK),
        several(SORT_ARRAY_RANK)
    );
    let ambiguous = ambiguous
        + "list_distinct(list_transform(list_filter(k, \
         lambda x: x['n'] IS NOT NULL), lambda x: x['n']))) <> len(list_distinct(list_transform(\
         list_filter(k, lambda x: x['n'] IS NOT NULL), lambda x: x['t'])))";
    let sorted = json_array("list_transform(list_sort(k), lambda x: CAST(x['t'] AS JSON))");
    bound(
        &format!("CAST({array} AS JSON)"),
        "q",
        &format!(
            "(CASE WHEN q IS NULL THEN NULL WHEN json_type(q) <> 'ARRAY' THEN {} \
             ELSE list_transform([{keyed}], lambda k: CASE WHEN {ambiguous} \
             THEN CAST({unordered} AS JSON) ELSE {sorted} END)[1] END)",
            json_error(NOT_AN_ARRAY_ARGUMENT, NOT_AN_ARRAY_MESSAGE)
        ),
    )
}
