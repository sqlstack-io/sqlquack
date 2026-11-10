//! `save_replay_cache`: this run's judgements, for the next run to reuse.

use std::path::Path;

use crate::constants::REPLAY_CACHE_FORMAT;
use crate::errors::EvidenceError;
use crate::verify::models::ReplayCache;

/// Writes the judgements `cache` collected to `path`, replacing the file; returns how many.
pub fn save_replay_cache(path: &Path, cache: &ReplayCache) -> Result<usize, EvidenceError> {
    let made = cache
        .made
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let file = serde_json::json!({ "format": REPLAY_CACHE_FORMAT, "judgements": *made });
    let text = serde_json::to_string(&file)
        .map_err(|error| EvidenceError::Io(format!("{}: {error}", path.display())))?;
    std::fs::write(path, text)
        .map_err(|error| EvidenceError::Io(format!("{}: {error}", path.display())))?;
    Ok(made.len())
}
