//! Constants of SQLite SQL's fingerprint profile.

/// Generic keywords SQLite lets name columns and tables: `ANY` and `SOME` are no SQLite keywords, and `FOR` and `WITH` fall back to names.
pub(crate) const NAME_KEYWORDS: [&str; 4] = ["ANY", "FOR", "SOME", "WITH"];
/// The keyword after `FOR` in a trigger's `FOR EACH ROW`.
pub(crate) const FOR_PARTNER: &str = "EACH";
/// The keyword after `WITH` in `WITH RECURSIVE`.
pub(crate) const WITH_PARTNER: &str = "RECURSIVE";
/// What follows a common table expression's name: its body's `AS` or its column list.
pub(crate) const CTE_NAME_FOLLOWERS: [&str; 2] = ["AS", "("];
