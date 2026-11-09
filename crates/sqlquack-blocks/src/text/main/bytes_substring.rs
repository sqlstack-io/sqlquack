//! `bytes_substring`: SUBSTR of bytes held as hexadecimal digits, counted in bytes.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::text::constants::BYTES_SUBSTRING;

/// Start 0 counts as 1, a negative start counts from the end and before the bytes gives none.
pub fn bytes_substring(
    value: &str,
    (start, length): (&str, Option<&str>),
    log: &ProvenanceLog,
) -> String {
    log.building_block(BYTES_SUBSTRING);
    let count = "length(p['h']) // 2";
    let length = length
        .map(|length| format!(", 2 * {length}"))
        .unwrap_or_default();
    format!(
        "list_transform([struct_pack(h := {value}, s := {start})], lambda p: CASE \
         WHEN p['h'] IS NULL OR p['s'] IS NULL THEN NULL \
         WHEN p['s'] < 0 AND -p['s'] > {count} THEN '' \
         ELSE substring(p['h'], 2 * (CASE WHEN p['s'] = 0 THEN 1 WHEN p['s'] < 0 \
         THEN {count} + p['s'] + 1 ELSE p['s'] END) - 1{length}) END)[1]"
    )
}
