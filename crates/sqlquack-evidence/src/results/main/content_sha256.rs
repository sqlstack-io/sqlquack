//! `content_sha256`: the digest that names recorded files and statements.

use crate::results::_helpers::reading::sha256_hex;

/// SHA-256 of `bytes`, as lower-case hex.
pub fn content_sha256(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}
