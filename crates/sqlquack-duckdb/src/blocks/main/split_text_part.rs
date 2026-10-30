//! `split_text_part`: SPLIT_PART of text, the text bound once.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::_helpers::text::once_bound;
use crate::blocks::constants::SPLIT_TEXT_PART;

/// The part at `part` (0 read as 1) of `text` split on `delimiter`, long text bound once for the macro.
pub fn split_text_part(text: &str, (delimiter, part): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(SPLIT_TEXT_PART);
    once_bound(text, "part_text", |text| {
        format!("split_part({text}, {delimiter}, (CASE WHEN {part} = 0 THEN 1 ELSE {part} END))")
    })
}
