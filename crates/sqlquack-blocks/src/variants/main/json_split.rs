//! `json_split`: SPLIT of text into a JSON array of strings.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::INLINE_ARGUMENT_LIMIT;
use crate::semi_structured::constants::JSON_SPLIT;
use crate::variants::_helpers::objects::bound;

/// The parts of `text` between occurrences of `separator`; an empty separator keeps the text whole.
pub fn json_split((text, separator): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_SPLIT);
    let split = |text: &str| {
        format!(
            "(CASE WHEN {text} IS NULL OR {separator} IS NULL THEN NULL \
             WHEN {separator} = '' THEN CAST(to_json([{text}]) AS JSON) \
             ELSE CAST(to_json(string_split({text}, {separator})) AS JSON) END)"
        )
    };
    match text.len() > INLINE_ARGUMENT_LIMIT {
        true => bound(text, "split_text", &split("split_text")),
        false => split(text),
    }
}
