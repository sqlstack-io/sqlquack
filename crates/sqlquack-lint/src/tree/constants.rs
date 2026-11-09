//! Constants of tree.

/// Window-only calls SQLQuack's Snowflake parser keeps as written aggregates; polyglot never counted them.
pub(crate) const WINDOW_ONLY_CALLS: [&str; 2] = ["FIRST_VALUE", "LAST_VALUE"];
/// The SQL/JSON aggregates the native PostgreSQL parser keeps as keyword calls.
pub(crate) const JSON_AGGREGATE_KINDS: [&str; 2] = ["JSON_ARRAYAGG", "JSON_OBJECTAGG"];
