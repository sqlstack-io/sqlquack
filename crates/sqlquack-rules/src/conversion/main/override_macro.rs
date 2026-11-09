//! `override_macro`: the DuckDB macro standing for a user override in converted SQL.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_blocks::blocks::main::quoted_identifier::quoted_identifier;

use crate::conversion::main::override_macro_name::override_macro_name;
use crate::conversion::models::UserOverride;

/// `CREATE OR REPLACE MACRO` for `replacement`, its parameters as the body reads them.
pub fn override_macro(replacement: &UserOverride) -> String {
    let parameters: Vec<String> = replacement
        .arguments
        .iter()
        .map(|argument| argument.name.to_ascii_lowercase())
        .collect();
    format!(
        "CREATE OR REPLACE MACRO {}({}) AS ({})",
        quoted_identifier(
            &override_macro_name(&replacement.name),
            &ProvenanceLog::default()
        ),
        parameters.join(", "),
        replacement.body
    )
}
