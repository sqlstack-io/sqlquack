use crate::check::models::{LocalFailure, LocalVerdict};

/// The verdict of a worker that failed with `failure`.
pub(crate) fn failed(failure: LocalFailure) -> LocalVerdict {
    LocalVerdict::Failed { failure }
}

/// `SELECT` of 1 inside 2,000 parentheses, past every parser's depth limit.
pub(crate) fn deep() -> String {
    format!("SELECT {}1{}", "(".repeat(2000), ")".repeat(2000))
}

/// A compound SELECT of 600 terms, past SQLite's `SQLITE_MAX_COMPOUND_SELECT` of 500.
pub(crate) fn many_terms() -> String {
    vec!["SELECT 1"; 600].join(" UNION ALL ")
}

/// A statement every dialect refuses as a syntax error.
pub(crate) fn syntax_error() -> String {
    "SELECT (1".to_owned()
}
