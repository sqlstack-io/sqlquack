//! `guarded_pattern`: a REGEXP pattern from a column, raising outside the recorded syntax.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::raises::main::raised::raised;
use crate::text::constants::{GUARDED_PATTERN, RECORDED_PATTERN_SYNTAX, REGEXP_PATTERN_SYNTAX};

/// The pattern `sql` behind the inline `flags`, raising for syntax no recording shows read alike.
pub fn guarded_pattern(sql: &str, flags: &str, log: &ProvenanceLog) -> String {
    log.building_block(GUARDED_PATTERN);
    format!(
        "list_transform([{sql}], lambda regexp_pattern: CASE WHEN regexp_pattern IS NULL THEN NULL \
         WHEN regexp_full_match(regexp_pattern, '{}') THEN '{flags}' || regexp_pattern ELSE CAST({} AS VARCHAR) END)[1]",
        RECORDED_PATTERN_SYNTAX.replace('\'', "''"),
        raised(&format!(
            "{REGEXP_PATTERN_SYNTAX}: a REGEXP pattern from a column in a syntax no recording shows"
        ))
    )
}
