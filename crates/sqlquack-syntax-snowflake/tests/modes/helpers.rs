use std::sync::mpsc;
use std::time::Duration;

use sqlquack_syntax::lossless::main::lossless_tokens::lossless_tokens;
use sqlquack_syntax::tokens::models::{Span, TokenType};
use sqlquack_syntax::{Dialect, ParseOptions};
use sqlquack_syntax_snowflake::{
    parse_statements, parse_strict, parse_tolerant, tokenize, StrictFailure,
};

pub(crate) fn strict_outcome(sql: &str) -> Result<usize, bool> {
    parse_strict(sql, Dialect::Snowflake, &ParseOptions::default())
        .map(|statements| statements.len())
        .map_err(|failure| matches!(failure, StrictFailure::Parse(_)))
}

pub(crate) fn tolerant_ranges(sql: &str, dialect: Dialect) -> Vec<(String, bool)> {
    parse_tolerant(sql, dialect, &ParseOptions::default())
        .statements
        .iter()
        .map(|statement| {
            (
                sql[statement.start..statement.end].to_owned(),
                statement.parsed.is_ok(),
            )
        })
        .collect()
}

pub(crate) fn tolerant_tokens(sql: &str, dialect: Dialect) -> (String, usize, bool) {
    let parse = parse_tolerant(sql, dialect, &ParseOptions::default());
    (
        parse.tokens.iter().map(|token| token.text(sql)).collect(),
        parse.tokens.len(),
        parse.token_failure.is_some(),
    )
}

pub(crate) fn tolerant_joined(sql: &str, dialect: Dialect) -> String {
    parse_tolerant(sql, dialect, &ParseOptions::default())
        .tokens
        .iter()
        .map(|token| token.text(sql))
        .collect()
}

pub(crate) fn tolerant_finishes_within(sql: &'static str, seconds: u64) -> bool {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        [Dialect::Snowflake, Dialect::DuckDB]
            .into_iter()
            .for_each(|dialect| drop(parse_tolerant(sql, dialect, &ParseOptions::default())));
        sender.send(()).ok();
    });
    receiver.recv_timeout(Duration::from_secs(seconds)).is_ok()
}

/// `parse_strict`'s outcome, written out.
pub(crate) fn strict_written(sql: &str, dialect: Dialect) -> String {
    format!("{:?}", parse_strict(sql, dialect, &ParseOptions::default()))
}

/// The parse, then a separate lossless tokenize of `sql`, as strict parsing first did, written out.
pub(crate) fn parse_then_tokenize_written(sql: &str, dialect: Dialect) -> String {
    let outcome: Result<_, StrictFailure> =
        parse_statements(sql, dialect, &ParseOptions::default())
            .map_err(StrictFailure::from)
            .and_then(|statements| {
                tokenize(sql, dialect)
                    .map(|_| statements)
                    .map_err(StrictFailure::from)
            });
    format!("{outcome:?}")
}

/// `lossless_tokens` over `spans` of `sql`, character offsets of variable tokens, written out.
pub(crate) fn coverage_written(sql: &str, spans: &[(usize, usize)]) -> String {
    let significant: Vec<(Span, TokenType)> = spans
        .iter()
        .map(|(start, end)| {
            (
                Span {
                    start: *start,
                    end: *end,
                    ..Span::default()
                },
                TokenType::Var,
            )
        })
        .collect();
    format!(
        "{:?}",
        lossless_tokens(sql, &significant, false).map(|tokens| tokens.len())
    )
}
