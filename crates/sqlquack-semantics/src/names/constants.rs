//! Refusal texts for relation names.

/// How a refusal for a relation without known columns begins, before the relation's name.
pub const UNKNOWN_RELATION_PREFIX: &str = "relation `";
/// How it ends, after the name.
pub const UNKNOWN_RELATION_SUFFIX: &str = "` has no known columns";
