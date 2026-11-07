//! Whitespace-, comment- and keyword-case-insensitive SQL change fingerprints over SQLQuack's tokenizers.

#![forbid(unsafe_code)]

pub mod fingerprint;

pub use crate::fingerprint::errors::FingerprintError;
pub use crate::fingerprint::main::canonical_tokens::canonical_tokens;
pub use crate::fingerprint::main::is_reserved_keyword::is_reserved_keyword;
pub use crate::fingerprint::main::query_fingerprint::query_fingerprint;
pub use crate::fingerprint::models::FingerprintDialect;
