//! `variant_json_branch`: a JSON-held set operation branch beside a FLOAT-built VARIANT.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::held::_helpers::guards::{bound, fidelity_error};
use crate::held::constants::VARIANT_JSON_BRANCH;
use crate::held::main::variant_built::variant_built;
use crate::held::main::variant_json_leaf::variant_json_leaf;

/// The JSON `sql` held as a VARIANT, raising for a JSON null, which DuckDB's VARIANT reads as NULL.
pub fn variant_json_branch(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(VARIANT_JSON_BRANCH);
    bound(
        &format!("CAST({sql} AS JSON)"),
        "branch_json",
        &format!(
            "(CASE WHEN json_type(branch_json) = 'NULL' THEN CAST(CAST({} AS JSON) AS VARIANT) \
             ELSE {} END)",
            fidelity_error(
                "a JSON null beside a FLOAT in one VARIANT, which would become SQL NULL"
            ),
            variant_built(&variant_json_leaf("branch_json", log), log)
        ),
    )
}
