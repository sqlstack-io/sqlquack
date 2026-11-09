//! `character_similarity`: Snowflake's JAROWINKLER_SIMILARITY, over characters and ignoring case.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::raises::main::raised::raised;
use crate::text::constants::{CHARACTER_SIMILARITY, SIMILARITY_ALPHABET, TEXT_SIMILARITY};

/// The Jaro-Winkler similarity of two texts' lower-cased characters, 0 to 100 rounded down.
pub fn character_similarity(left: &str, right: &str, log: &ProvenanceLog) -> String {
    log.building_block(CHARACTER_SIMILARITY);
    let folded = raised(&format!(
        "{TEXT_SIMILARITY}: a text holding a cased letter outside ASCII, whose folding no recording shows"
    ));
    let wide = raised(&format!(
        "{TEXT_SIMILARITY}: texts with more than {SIMILARITY_ALPHABET} distinct characters"
    ));
    let cased = |side: &str| {
        format!(
            "len(list_filter(string_split(p['{side}'], ''), lambda c: ascii(c) > 127 \
             AND (lower(c) <> c OR upper(c) <> c))) > 0"
        )
    };
    let encoded = |side: &str| {
        format!(
            "array_to_string(list_transform(string_split(lower(p['{side}']), ''), \
             lambda c: chr(list_position(a, c))), '')"
        )
    };
    format!(
        "list_transform([struct_pack(l := {left}, r := {right})], lambda p: CASE \
         WHEN p['l'] IS NULL OR p['r'] IS NULL THEN NULL \
         WHEN p['l'] = '' OR p['r'] = '' THEN CAST(0 AS DECIMAL(4,0)) \
         WHEN {} OR {} THEN CAST({folded} AS DECIMAL(4,0)) \
         ELSE list_transform([list_distinct(string_split(lower(p['l'] || p['r']), ''))], \
         lambda a: CASE WHEN len(a) > {SIMILARITY_ALPHABET} THEN CAST({wide} AS DECIMAL(4,0)) \
         ELSE CAST(floor(jaro_winkler_similarity({}, {}) * 100 + 1e-9) AS DECIMAL(4,0)) END)[1] \
         END)[1]",
        cased("l"),
        cased("r"),
        encoded("l"),
        encoded("r"),
    )
}
