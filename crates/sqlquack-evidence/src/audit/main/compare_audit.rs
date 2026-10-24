//! `compare_audit`: fresh recordings against the stored results of the same cases.

use std::path::Path;

use sqlquack_compare::comparison::main::compare_recordings::compare_recordings;
use sqlquack_compare::comparison::models::RowOrder;

use crate::audit::models::{AuditEntry, AuditOutcome};
use crate::cases::models::LoadedCase;
use crate::errors::EvidenceError;
use crate::results::main::load_result::load_result;
use crate::results::models::StoredResult;
use crate::verify::_helpers::listing::recorded_hashes;

/// One entry per fresh result in `fresh`, compared with the stored result in `stored`.
pub fn compare_audit(
    cases: &[LoadedCase],
    stored: &Path,
    fresh: &Path,
) -> Result<Vec<AuditEntry>, EvidenceError> {
    Ok(recorded_hashes(fresh)?
        .into_iter()
        .map(|hash| {
            let case = cases.iter().find(|case| case.hash == hash);
            let order = case.map_or(RowOrder::Ordered, |case| case.order);
            let stored_result = load_result(stored, &hash);
            let fresh_result = load_result(fresh, &hash);
            AuditEntry {
                reference: case.map(|case| case.reference.clone()),
                stored_warehouse_version: warehouse_version(&stored_result),
                fresh_warehouse_version: warehouse_version(&fresh_result),
                outcome: outcome(stored_result, fresh_result, order),
                hash,
            }
        })
        .collect())
}

fn warehouse_version(result: &Result<Option<StoredResult>, EvidenceError>) -> Option<String> {
    result
        .as_ref()
        .ok()
        .and_then(Option::as_ref)
        .map(|stored| stored.metadata.warehouse_version.clone())
}

fn outcome(
    stored: Result<Option<StoredResult>, EvidenceError>,
    fresh: Result<Option<StoredResult>, EvidenceError>,
    order: RowOrder,
) -> AuditOutcome {
    let (stored, fresh) = match (stored, fresh) {
        (Ok(Some(stored)), Ok(Some(fresh))) => (stored, fresh),
        (Ok(None), _) => {
            return AuditOutcome::Invalid {
                detail: "no stored result for this hash".to_owned(),
            };
        }
        (Err(error), _) | (_, Err(error)) => {
            return AuditOutcome::Invalid {
                detail: error.to_string(),
            };
        }
        (_, Ok(None)) => {
            return AuditOutcome::Invalid {
                detail: "the fresh result disappeared".to_owned(),
            };
        }
    };
    match (&stored.rows, &fresh.rows) {
        (Some(stored_rows), Some(fresh_rows)) => {
            match compare_recordings(stored_rows, fresh_rows, order) {
                Ok(()) => AuditOutcome::Matched,
                Err(mismatch) => AuditOutcome::Differed {
                    detail: mismatch.to_string(),
                },
            }
        }
        (None, None) if same_error(&stored, &fresh) => AuditOutcome::Matched,
        _ => AuditOutcome::Differed {
            detail: format!(
                "stored outcome {:?} ({:?}), fresh outcome {:?} ({:?})",
                stored.metadata.outcome,
                stored.metadata.error.as_ref().map(|error| &error.sqlstate),
                fresh.metadata.outcome,
                fresh.metadata.error.as_ref().map(|error| &error.sqlstate)
            ),
        },
    }
}

fn same_error(stored: &StoredResult, fresh: &StoredResult) -> bool {
    let key = |result: &StoredResult| {
        result
            .metadata
            .error
            .as_ref()
            .map(|error| (error.errno, error.sqlstate.clone()))
    };
    key(stored) == key(fresh)
}
