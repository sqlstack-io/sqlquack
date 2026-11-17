use crate::check::models::{LocalFailure, LocalVerdict};

/// The verdict of a worker that failed with `failure`.
pub(crate) fn failed(failure: LocalFailure) -> LocalVerdict {
    LocalVerdict::Failed { failure }
}

/// `SELECT` of 1 inside 2,000 parentheses, past every parser's depth limit.
pub(crate) fn deep() -> String {
    format!("SELECT {}1{}", "(".repeat(2000), ")".repeat(2000))
}

/// `SELECT 1 + 1 …` of 1,500 terms, past the chain-length limit of each parser that has one.
pub(crate) fn chain() -> String {
    format!("SELECT {}", vec!["1"; 1500].join(" + "))
}

/// A WHERE of 9,000 `OR` terms, past every native parser's tree limit.
pub(crate) fn tree() -> String {
    format!("SELECT 1 WHERE {}", vec!["1 = 1"; 9000].join(" OR "))
}

/// A compound SELECT of 600 terms, past SQLite's `SQLITE_MAX_COMPOUND_SELECT` of 500.
pub(crate) fn many_terms() -> String {
    vec!["SELECT 1"; 600].join(" UNION ALL ")
}

/// A statement every dialect refuses as a syntax error.
pub(crate) fn syntax_error() -> String {
    "SELECT (1".to_owned()
}
