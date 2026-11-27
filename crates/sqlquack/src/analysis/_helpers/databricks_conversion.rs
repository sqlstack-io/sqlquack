//! A case recorded on Databricks SQL, converted by the Databricks rules or refused.

use std::collections::BTreeMap;

use sqlquack_convert::databricks::analysis::main::analyze::analyze;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_evidence::verify::models::Conversion;

use crate::analysis::constants::{
    CASE_SETUP_UNREPLAYED, DATABRICKS_CASE_EXTRAS, STATEMENT_REFUSAL,
};
use crate::analysis::main::refused_construct::refused_construct;

/// The case's one statement as DuckDB SQL, or why it refuses.
pub(crate) fn databricks_conversion(case: &LoadedCase) -> Conversion {
    let unsupported = match (
        case.setup.is_empty(),
        case.functions.is_empty() && case.overrides.is_empty(),
    ) {
        (false, _) => Some(CASE_SETUP_UNREPLAYED),
        (true, false) => Some(DATABRICKS_CASE_EXTRAS),
        (true, true) => None,
    };
    if let Some(reason) = unsupported {
        return Conversion::Refused {
            reasons: vec![reason.to_owned()],
            constructs: vec![STATEMENT_REFUSAL.to_owned()],
        };
    }
    match analyze(&case.source_sql) {
        Ok(converted) => Conversion::Converted {
            sql: converted.sql,
            setup: Vec::new(),
            provenance: converted.provenance,
            raises: BTreeMap::new(),
            python: None,
        },
        Err(refusal) => Conversion::Refused {
            reasons: vec![format!("{}: {}", refusal.category, refusal.reason)],
            constructs: vec![refused_construct(&refusal)],
        },
    }
}
