//! Checks every corpus statement must pass whatever its verdict: lossless tokens, located parses.

use std::panic::{AssertUnwindSafe, catch_unwind};

use serde_json::Value;
use sqlquack_syntax::{CallMode, Dialect, Expression, ParseOptions, TokenizeFailure};
use sqlquack_syntax_snowflake::{parse_statements, parse_statements_with, tokenize};

use crate::verdicts::constants::SPAN_FIELD;
use crate::verdicts::models::StatementCheck;

/// Why the lossless tokens of `sql` do not rebuild it byte for byte, if they do not.
fn lossless_failure(sql: &str, dialect: Dialect, parsed: bool) -> Option<String> {
    let tokens = match catch_unwind(|| tokenize(sql, dialect)) {
        Ok(Ok(tokens)) => tokens,
        Ok(Err(TokenizeFailure::Tokenizer(_))) if !parsed => return None,
        Ok(Err(failure)) => return Some(failure.to_string()),
        Err(_) => return Some("the tokenizer panicked".to_owned()),
    };
    let mut cursor: usize = 0;
    for token in &tokens {
        if token.start != cursor || token.end <= token.start {
            return Some(format!(
                "token {}..{} does not continue at byte {cursor}",
                token.start, token.end
            ));
        }
        cursor = token.end;
    }
    match cursor == sql.len() {
        true => None,
        false => Some(format!("tokens end at byte {cursor} of {}", sql.len())),
    }
}

fn without_spans(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .filter(|(field, _)| field != SPAN_FIELD)
                .map(|(field, child)| (field, without_spans(child)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(without_spans).collect()),
        other => other,
    }
}

/// A parse result as comparable JSON without spans, or the error's debug form.
fn comparable(parsed: Result<Vec<Expression>, impl std::fmt::Debug>) -> Result<Value, String> {
    parsed
        .map_err(|error| format!("{error:?}"))
        .and_then(|trees| serde_json::to_value(trees).map_err(|error| error.to_string()))
        .map(without_spans)
}

/// Why the located parse of `sql` differs from `faithful` beyond spans, if it does.
fn located_difference(
    sql: &str,
    dialect: Dialect,
    faithful: Result<Vec<Expression>, impl std::fmt::Debug>,
) -> Option<String> {
    let located = catch_unwind(AssertUnwindSafe(|| {
        parse_statements_with(
            sql,
            dialect,
            &ParseOptions::default(),
            CallMode::FaithfulLocated,
        )
    }));
    let Ok(located) = located else {
        return Some("the located parse panicked".to_owned());
    };
    match comparable(faithful) == comparable(located) {
        true => None,
        false => Some("the located parse differs from the faithful parse".to_owned()),
    }
}

/// Whether SQLQuack refuses `sql` in `dialect`, and which invariants it breaks.
pub(crate) fn check_statement(sql: &str, dialect: Dialect) -> StatementCheck {
    let faithful = parse_statements(sql, dialect, &ParseOptions::default());
    let (refused, lossless) = match &faithful {
        Ok(_) => (false, lossless_failure(sql, dialect, true)),
        Err(_) => (true, lossless_failure(sql, dialect, false)),
    };
    let problems: Vec<String> = [lossless, located_difference(sql, dialect, faithful)]
        .into_iter()
        .flatten()
        .collect();
    StatementCheck { refused, problems }
}
