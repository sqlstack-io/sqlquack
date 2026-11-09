//! Dialect entries.

pub(crate) mod lint_dialect;
#[cfg(feature = "native-predicates")]
pub(crate) mod native_predicates;
#[cfg(feature = "native-parsers")]
pub(crate) mod native_statements;
pub(crate) mod profile;
#[cfg(feature = "shared-parser")]
pub(crate) mod shared_predicates;
#[cfg(feature = "shared-statements")]
pub(crate) mod shared_statements;
#[cfg(feature = "shared-parser")]
pub(crate) mod shared_tokens;
