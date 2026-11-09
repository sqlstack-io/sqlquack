use std::sync::mpsc;
use std::time::Duration;

use sqlquack_syntax::{Dialect, ParseOptions};
use sqlquack_syntax_snowflake::{parse_strict, parse_tolerant, StrictFailure};

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
