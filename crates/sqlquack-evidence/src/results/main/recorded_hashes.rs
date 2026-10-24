//! `recorded_hashes`: the case hashes recorded in a results directory.

use std::path::Path;

use crate::errors::EvidenceError;

/// Every hash with a metadata file in `results`, sorted; none when the directory is absent.
pub fn recorded_hashes(results: &Path) -> Result<Vec<String>, EvidenceError> {
    if !results.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(results)
        .map_err(|error| EvidenceError::Io(format!("{}: {error}", results.display())))?;
    let mut hashes: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .strip_suffix(".json")
                .map(str::to_owned)
        })
        .collect();
    hashes.sort();
    Ok(hashes)
}
