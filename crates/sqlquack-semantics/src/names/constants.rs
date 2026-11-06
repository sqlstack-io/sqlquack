//! Refusal texts for relation names.

/// How a refusal for a relation without known columns begins, before the relation's name.
pub const UNKNOWN_RELATION_PREFIX: &str = "relation `";
/// How it ends, after the name.
pub const UNKNOWN_RELATION_SUFFIX: &str = "` has no known columns";
/// What joins a schema key's parts: `STAGING.ORDERS` is the relation ORDERS in the schema STAGING.
pub const RELATION_KEY_SEPARATOR: &str = ".";
/// How a refusal for a name matching several schema relations ends, after the relations.
pub const AMBIGUOUS_RELATION_SUFFIX: &str =
    " in the schema; qualify it, or set the session's DATABASE and SCHEMA, to pick one";
