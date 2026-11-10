//! `load_replay_cache`: an earlier run's judgements, ready to reuse.

use std::path::Path;

use serde_json::Value;

use crate::constants::REPLAY_CACHE_FORMAT;
use crate::errors::EvidenceError;
use crate::verify::models::ReplayCache;

/// The judgements in the cache at `path` under `scope`; none when it is absent, unreadable or stale.
pub fn load_replay_cache(path: &Path, scope: &str) -> Result<ReplayCache, EvidenceError> {
    let empty = ReplayCache {
        scope: scope.to_owned(),
        ..ReplayCache::default()
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(empty),
        Err(error) => return Err(EvidenceError::Io(format!("{}: {error}", path.display()))),
    };
    let invalid =
        |error: serde_json::Error| EvidenceError::Io(format!("{}: {error}", path.display()));
    let mut file: Value = match serde_json::from_str(&text) {
        Ok(file) => file,
        Err(error) if !error.is_io() => return Ok(empty),
        Err(error) => return Err(invalid(error)),
    };
    if file.get("format").and_then(Value::as_str) != Some(REPLAY_CACHE_FORMAT) {
        return Ok(empty);
    }
    let known = match serde_json::from_value(file["judgements"].take()) {
        Ok(known) => known,
        Err(error) if !error.is_io() => return Ok(empty),
        Err(error) => return Err(invalid(error)),
    };
    Ok(ReplayCache { known, ..empty })
}
