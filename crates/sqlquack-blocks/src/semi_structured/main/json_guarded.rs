//! `json_guarded`: JSON text as a VARIANT, raising where DuckDB might read it differently.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::macros::constants::JSON_CHECKED_V1;
use crate::macros::main::helper_call::helper_call;
use crate::raises::constants::UNIDENTIFIED_SITE;
use crate::semi_structured::_helpers::paths::{path_list, plain_step, scoped_paths};
use crate::semi_structured::constants::JSON_GUARDED;
use crate::semi_structured::models::PathStep;

/// Text as DuckDB `JSON`, raising where read differently; only `reads`' paths are checked for repeats.
pub fn json_guarded(
    sql: &str,
    trying: bool,
    reads: Option<&[Vec<PathStep>]>,
    log: &ProvenanceLog,
) -> String {
    log.building_block(JSON_GUARDED);
    let (objects, values) = match reads {
        Some(paths) if paths.iter().flatten().all(plain_step) => scoped_paths(paths),
        _ => (Vec::new(), vec!["$".to_owned()]),
    };
    helper_call(
        JSON_CHECKED_V1,
        &[
            UNIDENTIFIED_SITE,
            &format!("CAST({sql} AS VARCHAR)"),
            if trying { "true" } else { "false" },
            &path_list(&objects),
            &path_list(&values),
        ],
    )
}
