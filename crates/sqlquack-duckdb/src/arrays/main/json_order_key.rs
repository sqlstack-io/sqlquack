//! `json_order_key`: a JSON-held VARIANT's sort key, in ARRAY_SORT's recorded order of kinds.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::arrays::constants::{
    ARRAY_SORT_ORDER, EXACT_DOUBLE_LIMIT, JSON_ORDER_KEY, SORT_KIND_ORDER,
};
use crate::raises::main::raised::raised;

/// Booleans, numbers by value, strings by bytes, then JSON null; objects and arrays raise unless ranked.
pub fn json_order_key(value: &str, containers_ranked: bool, log: &ProvenanceLog) -> String {
    log.building_block(JSON_ORDER_KEY);
    let mut ranks: Vec<String> = Vec::new();
    for (rank, kinds) in SORT_KIND_ORDER.iter().enumerate() {
        for kind in *kinds {
            ranks.push(format!("WHEN '{kind}' THEN {rank}"));
        }
    }
    let unordered = raised(&format!(
        "{ARRAY_SORT_ORDER}: a VARIANT object or array, or a fractional number beyond \
         {EXACT_DOUBLE_LIMIT}, as a sort key, whose order no recording shows"
    ));
    let (unranked, container_text) = match containers_ranked {
        true => ("", ", 'OBJECT', 'ARRAY'"),
        false => ("json_type(ok) IN ('OBJECT', 'ARRAY') OR ", ""),
    };
    format!(
        "list_transform([CAST({value} AS JSON)], lambda ok: CASE WHEN ok IS NULL THEN NULL \
         ELSE struct_pack(r := CASE WHEN {unranked}(json_type(ok) = 'DOUBLE' \
         AND abs(CAST(ok AS DOUBLE)) >= {EXACT_DOUBLE_LIMIT}) \
         THEN {unordered} ELSE CAST(CASE json_type(ok) {} END AS INTEGER) END, \
         b := CASE WHEN json_type(ok) = 'BOOLEAN' THEN CAST(ok AS BOOLEAN) END, \
         n := CASE WHEN json_type(ok) IN ('UBIGINT', 'BIGINT', 'DOUBLE') THEN CAST(ok AS DOUBLE) END, \
         i := CASE WHEN json_type(ok) IN ('UBIGINT', 'BIGINT') THEN CAST(ok AS HUGEINT) END, \
         s := CASE WHEN json_type(ok) = 'VARCHAR' THEN json_extract_string(ok, '$') \
         WHEN json_type(ok) IN (''{container_text}) THEN CAST(ok AS VARCHAR) END) END)[1]",
        ranks.join(" ")
    )
}
