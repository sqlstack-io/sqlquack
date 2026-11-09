//! `split_text_part`: SPLIT_PART of text, the text bound once.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::_helpers::text::once_bound;
use crate::blocks::constants::SPLIT_TEXT_PART;

/// The part at `part` (0 read as 1) of `text` split on `delimiter`; long text is bound with both.
pub fn split_text_part(text: &str, (delimiter, part): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(SPLIT_TEXT_PART);
    let split = |text: &str, delimiter: &str, part: &str| {
        format!("split_part({text}, {delimiter}, (CASE WHEN {part} = 0 THEN 1 ELSE {part} END))")
    };
    let bound = once_bound(text, "part_text", |name| name.to_owned());
    match bound == text {
        true => split(text, delimiter, part),
        false => format!(
            "list_transform([struct_pack(t := {text}, d := {delimiter}, n := {part})], \
             lambda part_text: {})[1]",
            split("part_text['t']", "part_text['d']", "part_text['n']")
        ),
    }
}
