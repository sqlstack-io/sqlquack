//! Comparing one fresh recording with its stored result.

use std::path::Path;

use sqlquack_compare::comparison::main::compare_recordings::compare_recordings;
use sqlquack_compare::comparison::models::RowOrder;

use crate::audit::models::{AuditEntry, AuditOutcome};
use crate::cases::models::LoadedCase;
use crate::errors::EvidenceError;
use crate::results::main::load_result::load_result;
use crate::results::models::{RecordedOutcome, StoredResult};

pub(crate) fn audit_entry(
    cases: &[LoadedCase],
    stored: &Path,
    fresh: &Path,
    hash: String,
) -> AuditEntry {
    let case: Option<&LoadedCase> = cases.iter().find(|case| case.hash == hash);
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
}

fn warehouse_version(result: &Result<Option<StoredResult>, EvidenceError>) -> Option<String> {
    match result {
        Ok(Some(stored)) => Some(stored.metadata.warehouse_version.clone()),
        Ok(None) | Err(_) => None,
    }
}

fn outcome(
    stored: Result<Option<StoredResult>, EvidenceError>,
    fresh: Result<Option<StoredResult>, EvidenceError>,
    order: RowOrder,
) -> AuditOutcome {
    let (stored, fresh) = match (stored, fresh) {
        (Ok(Some(stored)), Ok(Some(fresh))) => (stored, fresh),
        (Err(error), _) | (_, Err(error)) => {
            return AuditOutcome::Invalid {
                detail: error.to_string(),
            };
        }
        (Ok(None), _) | (_, Ok(None)) => {
            return AuditOutcome::Invalid {
                detail: "no stored result for this hash".to_owned(),
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
        (None, None)
            if stored.metadata.outcome == RecordedOutcome::Types
                && fresh.metadata.outcome == RecordedOutcome::Types =>
        {
            match (stored.metadata.columns == fresh.metadata.columns)
                && (stored.metadata.row_count == fresh.metadata.row_count)
            {
                true => AuditOutcome::Matched,
                false => AuditOutcome::Differed {
                    detail: "the recorded column types or row count differ".to_owned(),
                },
            }
        }
        (None, None) if error_key(&stored) == error_key(&fresh) => AuditOutcome::Matched,
        _ => AuditOutcome::Differed {
            detail: format!(
                "stored outcome {:?} {:?}, fresh outcome {:?} {:?}",
                stored.metadata.outcome,
                error_key(&stored),
                fresh.metadata.outcome,
                error_key(&fresh)
            ),
        },
    }
}

fn error_key(result: &StoredResult) -> Option<(Option<i64>, Option<String>)> {
    let error = result.metadata.error.as_ref()?;
    Some((error.errno, error.sqlstate.clone()))
}
