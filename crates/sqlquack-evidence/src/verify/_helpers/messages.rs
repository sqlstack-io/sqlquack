//! Snowflake's error message for a case recorded as an error.

use std::path::Path;

use crate::errors::EvidenceError;
use crate::results::main::load_result::load_result;

/// The stored error's message, else that of a re-recording raising the same errno and SQLSTATE.
pub(crate) fn recorded_message(
    hash: &str,
    results: &Path,
    audit_results: Option<&Path>,
) -> Result<Option<String>, EvidenceError> {
    let Some(stored) = load_result(results, hash)?.and_then(|stored| stored.metadata.error) else {
        return Ok(None);
    };
    if stored.message.is_some() {
        return Ok(stored.message);
    }
    let Some(directory) = audit_results.filter(|directory| directory.is_dir()) else {
        return Ok(None);
    };
    let audited = load_result(directory, hash)?.and_then(|audited| audited.metadata.error);
    Ok(audited
        .filter(|audited| audited.errno == stored.errno && audited.sqlstate == stored.sqlstate)
        .and_then(|audited| audited.message))
}
