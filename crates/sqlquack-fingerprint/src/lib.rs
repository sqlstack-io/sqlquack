//! Whitespace-, comment- and keyword-case-insensitive SQL change fingerprints over SQLQuack's tokenizers.

#![forbid(unsafe_code)]

#[cfg(feature = "bigquery")]
pub(crate) mod bigquery;
pub(crate) mod dialects;
#[cfg(feature = "duckdb")]
pub(crate) mod duckdb;
pub mod fingerprint;
#[cfg(feature = "postgres")]
pub(crate) mod postgres;
#[cfg(feature = "snowflake")]
pub(crate) mod snowflake;
#[cfg(feature = "sqlite")]
pub(crate) mod sqlite;
#[cfg(feature = "sqlserver")]
pub(crate) mod sqlserver;

pub use crate::fingerprint::errors::FingerprintError;
pub use crate::fingerprint::main::canonical_tokens::canonical_tokens;
pub use crate::fingerprint::main::is_reserved_keyword::is_reserved_keyword;
pub use crate::fingerprint::main::query_fingerprint::query_fingerprint;
pub use crate::fingerprint::models::FingerprintDialect;
