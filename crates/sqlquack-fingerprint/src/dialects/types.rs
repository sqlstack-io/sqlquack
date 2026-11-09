//! Types of the dialect profiles.

use crate::fingerprint::errors::FingerprintError;

/// The text of each significant token of some SQL in a dialect, or why its tokenizer stopped.
pub(crate) type TokenTexts = fn(&str) -> Result<Vec<&str>, FingerprintError>;
