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
        keywords.extend(case.else_span.0.map(Some));
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

/// The positions of clauses, list items, operators and arguments, each as its kind and text, sorted.
pub(crate) fn written_positions(statements: &[Expression], sql: &str) -> Vec<String> {
    let mut found: Vec<(&'static str, Option<Span>)> = Vec::new();
    for node in statements.iter().flat_map(|statement| statement.dfs()) {
        for with in node.with_clause().iter() {
            found.push(("with", with.keyword_span.0));
            found.extend(with.comma_spans.0.iter().map(|span| ("comma", Some(*span))));
        }
        found.extend(select_positions(node));
        found.extend(call_argument(node));
        found.extend(equality(node).map(|span| ("operation", Some(span))));
        found.extend(division(node).map(|span| ("operation", Some(span))));
        found.extend(membership(node).map(|span| ("operation", Some(span))));
        found.extend(range_test(node).map(|span| ("operation", Some(span))));
        found.extend(
            case_tests(node)
                .into_iter()
                .map(|span| ("when", Some(span))),
        );
        found.extend(deleted(node).map(|span| ("delete", Some(span))));
        found.extend(keyword_call(node).map(|span| ("call", Some(span))));
        found.extend(lateral_alias(node).map(|span| ("lateral", Some(span))));
    }
    let mut written: Vec<String> = found
        .into_iter()
        .filter_map(|(kind, span)| Some(format!("{kind} {}", written_at(sql, span?))))
        .collect();
    written.sort();
    written
}

fn select_positions(node: &Expression) -> Vec<(&'static str, Option<Span>)> {
    let Expression::Select(select) = node else {
        return Vec::new();
    };
    let mut found = vec![
        ("select", select.select_span.0),
        ("head", select.list_head_span.0),
        (
            "from",
            select.from.as_ref().and_then(|from| from.keyword_span.0),
        ),
        (
            "group",
            select
                .group_by
                .as_ref()
                .and_then(|group| group.keyword_span.0),
        ),
        (
            "order",
            select
                .order_by
                .as_ref()
                .and_then(|order| order.keyword_span.0),
        ),
    ];
    found.extend(
        select
            .item_spans
            .0
            .iter()
            .map(|item| ("item", item.operation())),
    );
    found.extend(
        select
            .alias_spans
            .0
            .iter()
            .map(|span| ("alias", Some(*span))),
    );
    found
}

fn call_argument(node: &Expression) -> Vec<(&'static str, Option<Span>)> {
    let Expression::Function(call) = node else {
        let Expression::AggregateFunction(call) = node else {
            return Vec::new();
        };
        return vec![("argument", call.argument_span.0)];
    };
    vec![("argument", call.argument_span.0)]
}

fn equality(node: &Expression) -> Option<Span> {
    let Expression::Eq(equal) = node else {
        return None;
    };
    equal.operator_span.operation()
}

fn division(node: &Expression) -> Option<Span> {
    let Expression::Div(quotient) = node else {
        return None;
    };
    quotient.operator_span.operation()
}

fn membership(node: &Expression) -> Option<Span> {
    let Expression::In(tested) = node else {
        return None;
    };
    tested.operator_span.operation()
}

fn range_test(node: &Expression) -> Option<Span> {
    let Expression::Between(tested) = node else {
        return None;
    };
    tested.operator_span.operation()
}

fn case_tests(node: &Expression) -> Vec<Span> {
    let Expression::Case(case) = node else {
        return Vec::new();
    };
    case.when_spans.0.clone()
}

fn deleted(node: &Expression) -> Option<Span> {
    let Expression::Delete(delete) = node else {
        return None;
    };
    delete.keyword_span.0
}

fn keyword_call(node: &Expression) -> Option<Span> {
    let Expression::KeywordCall(call) = node else {
        return None;
    };
    call.call_span.0
}

fn lateral_alias(node: &Expression) -> Option<Span> {
    let Expression::Lateral(lateral) = node else {
        return None;
    };
    lateral.alias_span.0
}
