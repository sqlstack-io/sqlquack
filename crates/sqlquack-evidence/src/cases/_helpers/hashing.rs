//! The content hash that names a case's result.

use sha2::{Digest, Sha256};

use crate::cases::models::HashedContent;

pub(crate) fn case_hash(content: &HashedContent<'_>) -> String {
    let canonical = serde_json::to_vec(content).unwrap_or_default();
    Sha256::digest(&canonical)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
