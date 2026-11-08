//! `query_fingerprint`: a whitespace-, comment- and keyword-case-insensitive SQL change fingerprint.

use sha2::{Digest, Sha256};

use crate::fingerprint::errors::FingerprintError;
use crate::fingerprint::main::canonical_tokens::canonical_tokens;
use crate::fingerprint::models::FingerprintDialect;

/// The SHA-256 hex digest of `sql`'s canonical tokens, each framed as `<byte length>:<text>`.
pub fn query_fingerprint(
    sql: &str,
    dialect: FingerprintDialect,
) -> Result<String, FingerprintError> {
    let mut digest = Sha256::new();
    for text in canonical_tokens(sql, dialect)? {
        digest.update(text.len().to_string().as_bytes());
        digest.update(b":");
        digest.update(text.as_bytes());
    }
    Ok(format!("{:x}", digest.finalize()))
}
