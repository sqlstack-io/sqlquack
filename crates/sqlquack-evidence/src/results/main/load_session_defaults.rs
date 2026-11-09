//! `load_session_defaults`: the recorded account defaults, when they have been recorded.

use std::path::Path;

use crate::errors::EvidenceError;
use crate::results::models::SessionDefaults;

/// The session defaults in `path`, `None` when the file does not exist.
pub fn load_session_defaults(path: &Path) -> Result<Option<SessionDefaults>, EvidenceError> {
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path)
        .map_err(|error| EvidenceError::Io(format!("{}: {error}", path.display())))?;
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|error| EvidenceError::InvalidResult(format!("{}: {error}", path.display())))
}
