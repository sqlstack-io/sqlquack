//! `folded_text`: text for ILIKE, its long s folded and letters no recording shows raising.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::raises::main::raised::raised;
use crate::text::constants::{CASE_FOLDING, FOLDED_TEXT, LONG_S, UNFOLDED_LETTERS};

/// The text with the long s as s; text holding a letter in `UNFOLDED_LETTERS` raises case_folding.
pub fn folded_text(value: &str, log: &ProvenanceLog) -> String {
    log.building_block(FOLDED_TEXT);
    let letters: String = UNFOLDED_LETTERS
        .iter()
        .map(|(first, last)| format!("\\x{{{first:X}}}-\\x{{{last:X}}}"))
        .collect();
    let unrecorded = raised(&format!(
        "{CASE_FOLDING}: case-insensitive text holding a letter whose folding no recording shows"
    ));
    format!(
        "list_transform([{value}], lambda folded: CASE WHEN regexp_matches(folded, '[{letters}]') \
         THEN CAST({unrecorded} AS VARCHAR) ELSE replace(folded, '{LONG_S}', 's') END)[1]"
    )
}
