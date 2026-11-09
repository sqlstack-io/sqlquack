//! Dialect entries.

pub(crate) mod comment;
#[cfg(feature = "native-scanner")]
pub(crate) mod gap_comments;
pub(crate) mod is_keyword;
pub(crate) mod is_unquoted_word;
pub(crate) mod profile;
#[cfg(feature = "native-scanner")]
pub(crate) mod shared_type;
