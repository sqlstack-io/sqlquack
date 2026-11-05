//! `content_sha256`: the lower-case hex SHA-256 every coverage hash uses.

use crate::formats::_helpers::hashes::sha256_hex;

/// The SHA-256 of `bytes` as lower-case hex.
pub fn content_sha256(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}
