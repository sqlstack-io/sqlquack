//! Constants of the dialect profiles.

/// The start of a comment that runs to the end of its line.
#[cfg(feature = "native-scanner")]
pub(crate) const LINE_COMMENT_START: &str = "--";
/// The start of a BigQuery comment that runs to the end of its line.
#[cfg(feature = "native-scanner")]
pub(crate) const HASH_COMMENT_START: char = '#';
/// The start of a `/* ... */` comment.
#[cfg(feature = "native-scanner")]
pub(crate) const BLOCK_COMMENT_START: &str = "/*";
/// The end of a `/* ... */` comment.
#[cfg(feature = "native-scanner")]
pub(crate) const BLOCK_COMMENT_END: &str = "*/";
