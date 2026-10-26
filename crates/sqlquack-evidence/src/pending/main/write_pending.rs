//! `write_pending`: a pending file for every case without a recorded result.

use std::collections::BTreeSet;
use std::path::Path;

use crate::cases::models::LoadedCase;
use crate::constants::{CASE_HASH_LENGTH, EVIDENCE_FORMAT, METADATA_EXTENSION, SOURCE_DIALECT};
use crate::errors::EvidenceError;
use crate::pending::models::{PendingCase, PendingSummary};

/// Writes `<hash>.json` into `out` for each unrecorded case and removes stale pending files.
pub fn write_pending(
    cases: &[LoadedCase],
    results: &Path,
    out: &Path,
    duckdb_version: &str,
) -> Result<PendingSummary, EvidenceError> {
    std::fs::create_dir_all(out)
        .map_err(|error| EvidenceError::Io(format!("{}: {error}", out.display())))?;
    let mut summary = PendingSummary::default();
    let mut wanted: BTreeSet<String> = BTreeSet::new();
    for case in cases {
        if results.join(format!("{}.json", case.hash)).exists() {
            summary.recorded += 1;
            continue;
        }
        let pending = PendingCase {
            format: EVIDENCE_FORMAT,
            case_hash: case.hash.clone(),
            case_reference: case.reference.clone(),
            source_dialect: SOURCE_DIALECT.to_owned(),
            session_parameters: case.session_parameters.clone(),
            pinned_parameters: case.pinned_parameters.clone(),
            source_sql: case.source_sql.clone(),
            duckdb_version: duckdb_version.to_owned(),
            values: (!case.values).then_some(false),
            setup_sql: case.setup_sql.clone(),
        };
        let name = format!("{}.json", case.hash);
        let text = serde_json::to_string_pretty(&pending)
            .map_err(|error| EvidenceError::Io(error.to_string()))?;
        std::fs::write(out.join(&name), text + "\n")
            .map_err(|error| EvidenceError::Io(format!("{name}: {error}")))?;
        wanted.insert(name);
        summary.written += 1;
    }
    summary.removed = remove_stale(out, &wanted)?;
    Ok(summary)
}

fn remove_stale(out: &Path, wanted: &BTreeSet<String>) -> Result<usize, EvidenceError> {
    let entries = std::fs::read_dir(out)
        .map_err(|error| EvidenceError::Io(format!("{}: {error}", out.display())))?;
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let stem = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let is_pending_file = path
            .extension()
            .is_some_and(|extension| extension == METADATA_EXTENSION)
            && stem.len() == CASE_HASH_LENGTH
            && stem.chars().all(|character| character.is_ascii_hexdigit());
        if is_pending_file && !wanted.contains(&name) {
            std::fs::remove_file(entry.path())
                .map_err(|error| EvidenceError::Io(format!("{name}: {error}")))?;
            removed += 1;
        }
    }
    Ok(removed)
}
