//! `load_recording`: a result recorded beside a project run, its sidecar and Arrow file checked.

use std::path::Path;

use sqlquack_compare::comparison::models::RecordedResult;

use crate::constants::ROWS_OUTCOME;
use crate::errors::EvidenceError;
use crate::results::_helpers::reading::{read_arrow, recorded_result, sha256_hex};
use crate::results::models::RecordingSidecar;

/// The rows recorded in `sidecar` and the Arrow file beside it, or why they cannot be compared.
pub fn load_recording(sidecar: &Path) -> Result<RecordedResult, EvidenceError> {
    let label = sidecar.display().to_string();
    let invalid = |message: String| EvidenceError::InvalidResult(format!("{label}: {message}"));
    let text = std::fs::read_to_string(sidecar).map_err(|error| invalid(error.to_string()))?;
    let metadata: RecordingSidecar =
        serde_json::from_str(&text).map_err(|error| invalid(error.to_string()))?;
    if metadata.outcome != ROWS_OUTCOME {
        return Err(invalid(format!(
            "outcome `{}` records no rows",
            metadata.outcome
        )));
    }
    let bytes = std::fs::read(sidecar.with_extension("arrow"))
        .map_err(|error| invalid(error.to_string()))?;
    if metadata.arrow_sha256.as_deref() != Some(sha256_hex(&bytes).as_str()) {
        return Err(invalid(
            "the Arrow file does not match the hash in its sidecar".to_owned(),
        ));
    }
    let rows = read_arrow(&label, bytes)?;
    recorded_result(&label, &metadata.columns, metadata.row_count, rows)
}
