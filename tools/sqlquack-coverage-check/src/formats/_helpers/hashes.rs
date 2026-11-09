//! Content hashes, and the exact text each case hash covers.

use std::collections::BTreeMap;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::formats::constants::{GIT_TREE_LENGTH, SHA256_LENGTH};

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// SHA-256 of the setup as compact JSON, which Python writes with `separators=(",", ":")`.
pub(crate) fn setup_sha256(setup: &[String]) -> String {
    sha256_hex(serde_json::to_string(setup).unwrap_or_default().as_bytes())
}

/// SHA-256 of the session as compact JSON with sorted keys.
pub(crate) fn session_sha256(session: &BTreeMap<String, Value>) -> String {
    sha256_hex(
        serde_json::to_string(session)
            .unwrap_or_default()
            .as_bytes(),
    )
}

fn lower_hex(text: &str, length: usize) -> bool {
    text.len() == length
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(crate) fn is_sha256(text: &str) -> bool {
    lower_hex(text, SHA256_LENGTH)
}

pub(crate) fn is_git_tree(text: &str) -> bool {
    lower_hex(text, GIT_TREE_LENGTH)
}
