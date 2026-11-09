//! Constants of tokens.

/// The text "e", as comparisons test it.
pub(crate) const LOWERCASE_ESCAPE_STRING_PREFIX: &str = "e";

/// Texts longer than this are never keywords.
pub(crate) const MAX_KEYWORD_LEN: usize = 128;

/// The quotes that open or close a triple-quoted string.
pub(crate) const TRIPLE_QUOTE_LEN: usize = 3;

/// The fewest tokens before raw data: `INSERT`, `FORMAT` and the format name.
pub(crate) const INSERT_FORMAT_MIN_TOKENS: usize = 3;
