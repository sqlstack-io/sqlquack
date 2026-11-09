//! `load_cases`: every case under the cases directory, validated against the policy.

use std::path::Path;

use sqlquack_policy::policy::models::Policy;

use crate::cases::_helpers::files::read_case_files;
use crate::cases::_helpers::loading::load_file;
use crate::cases::models::LoadedCase;
use crate::errors::EvidenceError;

/// Loads and validates every `.toml` case file under `directory`, sorted by reference.
pub fn load_cases(directory: &Path, policy: &Policy) -> Result<Vec<LoadedCase>, EvidenceError> {
    let mut cases: Vec<LoadedCase> = Vec::new();
    let mut problems: Vec<String> = Vec::new();
    for (relative, file) in read_case_files(directory)? {
        match load_file(&relative, &file, policy) {
            Ok(loaded) => cases.extend(loaded),
            Err(found) => problems.extend(found),
        }
    }
    if !problems.is_empty() {
        return Err(EvidenceError::InvalidCases(problems));
    }
    cases.sort_by(|left, right| left.reference.cmp(&right.reference));
    Ok(cases)
}
