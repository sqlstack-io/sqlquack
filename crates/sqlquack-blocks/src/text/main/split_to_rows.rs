//! `split_to_rows`: SPLIT_TO_TABLE's rows for a text, as a DuckDB subquery.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::text::constants::SPLIT_TO_ROWS;

/// INDEX from 1 and VALUE for each part of `text` split on the non-empty `delimiter`; NULL gives none.
pub fn split_to_rows(text: &str, delimiter: &str, log: &ProvenanceLog) -> String {
    log.building_block(SPLIT_TO_ROWS);
    format!(
        "SELECT CAST(generate_subscripts(_SPLIT_INPUT.parts, 1) AS DECIMAL(38,0)) AS \"INDEX\", \
         unnest(_SPLIT_INPUT.parts) AS \"VALUE\" \
         FROM (SELECT string_split({text}, {delimiter}) AS parts) AS _SPLIT_INPUT"
    )
}
