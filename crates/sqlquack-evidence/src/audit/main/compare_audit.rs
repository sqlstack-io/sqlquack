//! `compare_audit`: fresh recordings against the stored results of the same cases.

use std::path::Path;

use crate::audit::_helpers::outcomes::audit_entry;
use crate::audit::models::AuditEntry;
use crate::cases::models::LoadedCase;
use crate::errors::EvidenceError;
use crate::results::main::recorded_hashes::recorded_hashes;

/// One entry per fresh result in `fresh`, compared with the stored result in `stored`.
pub fn compare_audit(
    cases: &[LoadedCase],
    stored: &Path,
    fresh: &Path,
) -> Result<Vec<AuditEntry>, EvidenceError> {
    let mut entries: Vec<AuditEntry> = Vec::new();
    for hash in recorded_hashes(fresh)? {
        entries.push(audit_entry(cases, stored, fresh, hash));
    }
    Ok(entries)
}
