//! `variant_text`: a VARIANT-held value cast to text.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::floats::constants::FLOAT_TEXT;
use crate::floats::main::float_to_text::float_to_text;
use crate::held::_helpers::guards::bound;
use crate::held::_helpers::writing::held_written;
use crate::held::constants::VARIANT_TEXT;
use crate::raises::main::raised::raised;

/// The VARIANT-held `sql` as `::VARCHAR` writes it; a double read as an `element` has the FLOAT's text.
pub fn variant_text(sql: &str, element: bool, log: &ProvenanceLog) -> String {
    log.building_block(VARIANT_TEXT);
    let double = match element {
        true => float_to_text("CAST(held_value AS DOUBLE)", log),
        false => format!(
            "CAST({} AS VARCHAR)",
            raised(&format!(
                "{FLOAT_TEXT}: a FLOAT in a VARIANT cast to text, which Snowflake writes as Java \
                 does when it folds a constant"
            ))
        ),
    };
    bound(
        &format!("CAST({sql} AS JSON)"),
        "held_value",
        &format!(
            "(CASE json_type(held_value) WHEN 'VARCHAR' THEN json_extract_string(held_value, '$') \
             WHEN 'NULL' THEN NULL WHEN 'DOUBLE' THEN {double} ELSE {} END)",
            held_written("held_value", log)
        ),
    )
}
