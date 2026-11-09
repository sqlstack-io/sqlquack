//! `json_split`: SPLIT of text into a JSON array of strings.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::JSON_SPLIT;

/// The parts of `text` between occurrences of `separator`; an empty separator keeps the text whole.
pub fn json_split((text, separator): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_SPLIT);
    format!(
        "(CASE WHEN {text} IS NULL OR {separator} IS NULL THEN NULL \
         WHEN {separator} = '' THEN CAST(to_json([{text}]) AS JSON) \
         ELSE CAST(to_json(string_split({text}, {separator})) AS JSON) END)"
    )
}
