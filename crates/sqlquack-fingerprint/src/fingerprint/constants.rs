//! Tokens that decide whether a word folds.

/// The token that follows a called function's name.
pub(crate) const CALL_OPENER: &str = "(";
/// The token that closes a call's arguments or a column list.
pub(crate) const CALL_CLOSER: &str = ")";
/// Keywords after which a word followed by `(` names a relation and its column list.
pub(crate) const COLUMN_LIST_RELATION_KEYWORDS: [&str; 2] = ["INTO", "TABLE"];
/// The keyword between a CTE's column list and its body, and before an alias.
pub(crate) const ALIAS_KEYWORD: &str = "AS";
/// The tokens that can follow `AS` after a CTE's column list.
pub(crate) const CTE_BODY_OPENERS: [&str; 3] = ["(", "MATERIALIZED", "NOT"];
/// Tokens after which any word, even a reserved one, is a name.
pub(crate) const NAME_PREFIXES: [&str; 4] = [".", ":", "@", "$"];
/// The prefix of placeholder calls such as SQLBuild's `__ref` and `__source`.
pub(crate) const PLACEHOLDER_PREFIX: &str = "__";
