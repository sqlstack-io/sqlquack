//! Helpers of the format tests.

use serde_json::Value;
use sqlquack_syntax::{Dialect, TokenKind, tokenize};

use crate::format::constants::{DEPTH_FAILURE, PARSE_FAILURE, TYPE_DEPTH_FAILURE};
use crate::format::main::format_sql::format_sql;
use crate::format::models::FormatOptions;
use crate::format::tests::test_types::{CorpusOutcome, EntryOutcome, FormatCorpusTestCase};

/// The line width the corpus is also wrapped at, as SQLClean's corpus test does.
const CORPUS_LINE_WIDTH: usize = 40;
/// The corpus dialect names SQLQuack tokenizes, with their dialects.
const DIALECTS: [(&str, Dialect); 3] = [
    ("snowflake", Dialect::Snowflake),
    ("duckdb", Dialect::DuckDB),
    ("postgres", Dialect::PostgreSQL),
];

fn dialect(name: &str) -> Dialect {
    DIALECTS
        .iter()
        .find(|(known, _)| *known == name)
        .map_or(Dialect::Snowflake, |(_, dialect)| *dialect)
}

/// Format `sql`; `None` when the formatter refuses it.
fn format_once(sql: &str, dialect: Dialect, line_width: Option<usize>) -> Option<String> {
    let options = FormatOptions {
        line_width,
        ..FormatOptions::new(dialect)
    };
    format_sql(sql, &options)
        .ok()
        .map(|formatted| formatted.sql)
}

/// The significant token texts of `sql`.
fn token_texts(sql: &str, dialect: Dialect) -> Vec<String> {
    tokenize(sql, dialect)
        .unwrap_or_default()
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::Significant(_)))
        .map(|token| token.text(sql).to_string())
        .collect()
}

/// Whether `printed` has the tokens of `sql` except for words printed in upper case.
fn only_keyword_case_differs(sql: &str, printed: &str, dialect: Dialect) -> bool {
    let (before, after) = (token_texts(sql, dialect), token_texts(printed, dialect));
    before.len() == after.len()
        && before.iter().zip(&after).all(|(left, right)| {
            left == right
                || (left.eq_ignore_ascii_case(right) && *right == right.to_ascii_uppercase())
        })
}

/// Whether a layout of an earlier formatter release is relaid with its tokens kept.
fn relays_previous(previous: &str, dialect: Dialect) -> bool {
    format_once(previous, dialect, None)
        .is_some_and(|relaid| only_keyword_case_differs(previous, &relaid, dialect))
}

/// Whether formatting `stable` again leaves it as it is.
fn stays(stable: &str, dialect: Dialect, line_width: Option<usize>) -> bool {
    format_once(stable, dialect, line_width).as_deref() == Some(stable)
}

/// Format one corpus query and run every check SQLClean's corpus test runs on it.
fn check_entry(entry: &Value, test_case: &FormatCorpusTestCase) -> EntryOutcome {
    let name = test_case.dialect;
    let dialect = dialect(name);
    let sql = entry["sql"].as_str().unwrap_or_default();
    let departure = test_case
        .departures
        .iter()
        .find(|departure| departure.sql == sql);
    let recorded = entry["formatted"].as_str();
    let expected: Option<String> = departure
        .map_or(recorded, |departure| departure.formatted)
        .map(str::to_string);
    let receipt = departure.map(|departure| (departure.reason, departure.receipt));
    let actual = format_once(sql, dialect, None);
    let wrapped = format_once(sql, dialect, Some(CORPUS_LINE_WIDTH));
    let previous = entry["previously_formatted"].as_str();
    let checks = [
        (
            actual == expected,
            format!(
                "{name} layout differs (departure {receipt:?}):\n{sql}\n--- expected\n{}\n--- actual\n{}",
                expected.as_deref().unwrap_or("<refused>"),
                actual.as_deref().unwrap_or("<refused>")
            ),
        ),
        (
            departure.is_none_or(|departure| departure.formatted != recorded),
            format!("{name} departure matches SQLClean's layout: {sql}"),
        ),
        (
            only_keyword_case_differs(sql, actual.as_deref().unwrap_or(sql), dialect),
            format!("{name} tokens changed beyond keyword case: {sql}"),
        ),
        (
            previous.is_none_or(|previous| relays_previous(previous, dialect)),
            format!("{name} previous layout not relaid: {previous:?}"),
        ),
        (
            expected
                .as_deref()
                .is_none_or(|stable| stays(stable, dialect, None)),
            format!("{name} layout drifted: {sql}"),
        ),
        (
            wrapped.is_some() == actual.is_some(),
            format!("{name} wrapped outcome differs: {sql}"),
        ),
        (
            wrapped
                .as_deref()
                .is_none_or(|wrapped| stays(wrapped, dialect, Some(CORPUS_LINE_WIDTH))),
            format!("{name} wrapping drifted: {sql}"),
        ),
    ];
    EntryOutcome {
        formatted: actual.is_some(),
        departed: departure.is_some(),
        failures: checks
            .into_iter()
            .filter(|(passed, _)| !passed)
            .map(|(_, failure)| failure)
            .collect(),
    }
}

/// Format every corpus query of one dialect; return the formatted, refused and departing counts and failures.
pub(super) fn check_dialect_corpus(
    corpus: &[Value],
    test_case: &FormatCorpusTestCase,
) -> CorpusOutcome {
    let outcomes: Vec<EntryOutcome> = corpus
        .iter()
        .filter(|entry| entry["dialect"] == test_case.dialect)
        .map(|entry| check_entry(entry, test_case))
        .collect();
    let formatted = outcomes.iter().filter(|outcome| outcome.formatted).count();
    CorpusOutcome {
        formatted,
        refused: outcomes.len() - formatted,
        departed: outcomes.iter().filter(|outcome| outcome.departed).count(),
        failures: outcomes
            .into_iter()
            .flat_map(|outcome| outcome.failures)
            .collect(),
    }
}

/// The failure constant `reason` starts with, or all of `reason` when it starts with none of them.
pub(super) fn refusal_start(reason: &str) -> &str {
    [PARSE_FAILURE, DEPTH_FAILURE, TYPE_DEPTH_FAILURE]
        .into_iter()
        .find(|failure| reason.starts_with(failure))
        .unwrap_or(reason)
}
