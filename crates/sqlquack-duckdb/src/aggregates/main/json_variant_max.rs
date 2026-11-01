//! `json_variant_max`: MAX of JSON-held VARIANTs, raising where no recording shows their order.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::aggregates::constants::JSON_VARIANT_MAX;
use crate::arrays::constants::ARRAY_SORT_ORDER;
use crate::raises::main::raised::raised;

/// The one value, the largest of numbers distinct as doubles, or the bytewise largest string.
pub fn json_variant_max(value: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_VARIANT_MAX);
    let texts = "len(list_distinct(list_transform(vs, lambda e: CAST(e AS VARCHAR))))";
    let keyed = |kinds: &str, key: &str| {
        format!(
            "len(list_filter(vs, lambda e: json_type(e) NOT IN ({kinds}))) = 0 AND \
             len(list_distinct(list_transform(vs, lambda e: {key}))) = {texts} THEN \
             list_transform([list_transform(vs, lambda e: {key})], \
             lambda ks: vs[list_position(ks, list_max(ks))])[1]"
        )
    };
    format!(
        "list_transform([list(CAST({value} AS JSON)) FILTER (WHERE {value} IS NOT NULL)], \
         lambda vs: CASE WHEN coalesce(len(vs), 0) = 0 THEN NULL WHEN {texts} = 1 THEN vs[1] \
         WHEN {} WHEN {} ELSE CAST({} AS JSON) END)[1]",
        keyed("'UBIGINT', 'BIGINT', 'DOUBLE'", "CAST(e AS DOUBLE)"),
        keyed("'VARCHAR'", "json_extract_string(e, '$')"),
        raised(&format!(
            "{ARRAY_SORT_ORDER}: MAX of VARIANT values of different kinds, whose order no recording shows"
        ))
    )
}
