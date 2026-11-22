//! Helpers of the parsing tests.

use sqlquack_syntax::tokens::models::Span;
use sqlquack_syntax::{CallMode, Dialect, Expression, ExpressionWalk, ParseOptions};

/// The statements of `sql` with every call as written.
pub(crate) fn parse_as_written(sql: &str, dialect: Dialect) -> Option<Vec<Expression>> {
    sqlquack_syntax_snowflake::parse_statements_with(
        sql,
        dialect,
        &ParseOptions::default(),
        CallMode::AsWrittenLocated,
    )
    .ok()
}

/// The statements of `sql` with every query's own range.
pub(crate) fn parse_queries(sql: &str, dialect: Dialect) -> Option<Vec<Expression>> {
    sqlquack_syntax_snowflake::parse_statements_with(
        sql,
        dialect,
        &ParseOptions::default(),
        CallMode::QueriesLocated,
    )
    .ok()
}

/// Each CASE's keywords as written, by where the CASE starts.
pub(crate) fn case_keywords(statements: &[Expression], sql: &str) -> Vec<String> {
    let mut found: Vec<(usize, String)> = Vec::new();
    for node in statements.iter().flat_map(|statement| statement.dfs()) {
        let Expression::Case(case) = node else {
            continue;
        };
        let mut keywords: Vec<Option<Span>> = vec![case.case_keyword()];
        for branch in 0..case.whens.len() {
            let written: Option<(Span, Span)> = case.branch_keywords(branch);
            keywords.push(written.map(|(when, _)| when));
            keywords.push(written.map(|(_, then)| then));
        }
        keywords.extend(case.else_.as_ref().map(|_| case.else_keyword()));
        keywords.push(case.end_keyword());
        let words: Vec<String> = keywords
            .iter()
            .map(|span| span.map_or_else(|| "?".to_owned(), |span| written_at(sql, span)))
            .collect();
        let start: usize = case.case_keyword().map_or(0, |span| span.start);
        found.push((start, words.join(" ")));
    }
    found.sort();
    found.into_iter().map(|(_, words)| words).collect()
}

fn written_at(sql: &str, span: Span) -> String {
    sql.chars()
        .skip(span.start)
        .take(span.end - span.start)
        .collect()
}
