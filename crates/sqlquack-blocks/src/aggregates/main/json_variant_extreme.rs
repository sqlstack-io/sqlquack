//! `json_variant_extreme`: MIN or MAX of JSON-held VARIANTs in ARRAY_SORT's order of kinds.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::aggregates::constants::{EXTREME_KEY_TIES, JSON_VARIANT_EXTREME};
use crate::arrays::constants::ARRAY_SORT_ORDER;
use crate::arrays::main::json_order_key::json_order_key;
use crate::raises::main::raised::raised;

/// The extreme value, raising for several distinct objects or arrays, or ties written differently.
pub fn json_variant_extreme(value: &str, largest: bool, log: &ProvenanceLog) -> String {
    log.building_block(JSON_VARIANT_EXTREME);
    let extreme = match largest {
        true => "list_max",
        false => "list_min",
    };
    let several = |kind: &str| {
        format!(
            "len(list_distinct(list_transform(list_filter(vs, lambda c: json_type(c) = '{kind}'), \
             lambda c: CAST(c AS VARCHAR)))) > 1"
        )
    };
    let unordered = raised(&format!(
        "{ARRAY_SORT_ORDER}: MIN or MAX of several VARIANT objects or arrays, whose order no \
         recording shows"
    ));
    let tied = raised(&format!(
        "{EXTREME_KEY_TIES}: VARIANT values tied for MIN or MAX are written differently"
    ));
    format!(
        "list_transform([list(CAST({value} AS JSON)) FILTER (WHERE {value} IS NOT NULL)], \
         lambda vs: CASE WHEN coalesce(len(vs), 0) = 0 THEN NULL WHEN {} OR {} \
         THEN CAST({unordered} AS JSON) ELSE list_transform([list_transform(vs, \
         lambda x: {{'k': {}, 'v': x}})], lambda kr: list_transform([list_filter(kr, \
         lambda y: y['k'] = {extreme}(list_transform(kr, lambda z: z['k'])))], \
         lambda top: CASE WHEN len(list_distinct(list_transform(top, \
         lambda t: CAST(t['v'] AS VARCHAR)))) = 1 THEN top[1]['v'] \
         ELSE CAST({tied} AS JSON) END)[1])[1] END)[1]",
        several("OBJECT"),
        several("ARRAY"),
        json_order_key("x", true, log),
    )
}
