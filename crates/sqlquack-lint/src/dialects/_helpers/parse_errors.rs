//! Parse errors naming the line and column where a dialect's own parser stopped.

/// The parse error for a character at byte `position` that no token of the dialect covers.
pub(crate) fn uncovered(sql: &str, position: usize) -> String {
    parse_error(
        sql,
        position,
        "no token of the dialect covers this character",
    )
}

/// A parse error naming the one-based line and character column of byte `position`.
pub(crate) fn parse_error(sql: &str, position: usize, message: &str) -> String {
    let before: &str = sql.get(..position).unwrap_or(sql);
    let line: usize = before.matches('\n').count() + 1;
    let column: usize = before
        .rsplit('\n')
        .next()
        .map_or(0, |text| text.chars().count())
        + 1;
    format!("Parse error at line {line}, column {column}: {message}")
}
