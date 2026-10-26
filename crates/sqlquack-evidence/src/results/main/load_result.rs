//! `load_result`: one stored result, checked against its name and its Arrow file.

use std::path::Path;

use crate::errors::EvidenceError;
use crate::results::_helpers::reading::{read_arrow, read_metadata, recorded_result, sha256_hex};
use crate::results::models::{RecordedOutcome, StoredResult};

/// The result named `hash` in `directory`, `None` when nothing is recorded yet.
pub fn load_result(directory: &Path, hash: &str) -> Result<Option<StoredResult>, EvidenceError> {
    let metadata_path = directory.join(format!("{hash}.json"));
    let arrow_path = directory.join(format!("{hash}.arrow"));
    if !metadata_path.exists() {
        if arrow_path.exists() {
            return Err(EvidenceError::InvalidResult(format!(
                "{hash}: an Arrow file without metadata"
            )));
        }
        return Ok(None);
    }
    let metadata = read_metadata(&metadata_path)?;
    if metadata.case_hash != hash {
        return Err(EvidenceError::InvalidResult(format!(
            "{hash}: the metadata names case {}",
            metadata.case_hash
        )));
    }
    let rows = match metadata.outcome {
        RecordedOutcome::Error => {
            if arrow_path.exists() || metadata.error.is_none() {
                return Err(EvidenceError::InvalidResult(format!(
                    "{hash}: an error outcome needs an error and no Arrow file"
                )));
            }
            None
        }
        RecordedOutcome::Types => {
            if arrow_path.exists() || metadata.arrow_sha256.is_some() || metadata.error.is_some() {
                return Err(EvidenceError::InvalidResult(format!(
                    "{hash}: a types outcome has no Arrow file and no error"
                )));
            }
            None
        }
        RecordedOutcome::Rows => {
            let bytes = std::fs::read(&arrow_path).map_err(|error| {
                EvidenceError::InvalidResult(format!("{hash}: cannot read the Arrow file: {error}"))
            })?;
            if metadata.arrow_sha256.as_deref() != Some(sha256_hex(&bytes).as_str()) {
                return Err(EvidenceError::InvalidResult(format!(
                    "{hash}: the Arrow file does not match the hash in its metadata"
                )));
            }
            Some(recorded_result(&metadata, read_arrow(hash, bytes)?)?)
        }
    };
    Ok(Some(StoredResult { metadata, rows }))
}
