//! Helpers of the format tests.

use serde_json::Value;
use sqlquack_parse::tokenize;
use sqlquack_syntax::TokenKind;

use crate::dialects::main::profile::profile;
use crate::format::_helpers::lexemes::lex;
use crate::format::constants::{DEPTH_FAILURE, PARSE_FAILURE, TYPE_DEPTH_FAILURE};
use crate::format::main::format_sql::format_sql;
use crate::format::models::{FormatDialect, FormatOptions};
use crate::format::tests::test_types::{CorpusOutcome, Departure, EntryOutcome};

/// The format corpus every dialect's corpus test reads its queries from.
const FORMAT_CORPUS: &str = include_str!("fixtures/format_corpus.json");
/// The line width the corpus is also wrapped at, as SQLClean's corpus test does.
const CORPUS_LINE_WIDTH: usize = 40;
/// Format `sql`; `None` when the formatter refuses it.
fn format_once(sql: &str, dialect: FormatDialect, line_width: Option<usize>) -> Option<String> {
    let options = FormatOptions {
        line_width,
        ..FormatOptions::new(dialect)
    };
    format_sql(sql, &options)
        .ok()
        .map(|formatted| formatted.sql)
}

/// The significant token texts of `sql`: the dialect's own scanner's, or the shared tokenizer's.
fn token_texts(sql: &str, dialect: FormatDialect) -> Vec<String> {
    let Ok(profile) = profile(dialect) else {
        return Vec::new();
    };
    let None = profile.lex else {
        return lex(sql, &profile)
            .map(|(lexemes, _)| lexemes.into_iter().map(|lexeme| lexeme.raw).collect())
            .unwrap_or_default();
    };
    tokenize(sql, profile.dialect)
        .unwrap_or_default()
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::Significant(_)))
        .map(|token| token.text(sql).to_string())
        .collect()
}

/// Whether `printed` has the tokens of `sql` except for words printed in upper case.
fn only_keyword_case_differs(sql: &str, printed: &str, dialect: FormatDialect) -> bool {
    let (before, after) = (token_texts(sql, dialect), token_texts(printed, dialect));
    before.len() == after.len()
        && before.iter().zip(&after).all(|(left, right)| {
            left == right
                || (left.eq_ignore_ascii_case(right) && *right == right.to_ascii_uppercase())
        })
}

/// Whether a layout of an earlier formatter release is relaid with its tokens kept.
fn relays_previous(previous: &str, dialect: FormatDialect) -> bool {
    format_once(previous, dialect, None)
        .is_some_and(|relaid| only_keyword_case_differs(previous, &relaid, dialect))
}

/// Whether formatting `stable` again leaves it as it is.
fn stays(stable: &str, dialect: FormatDialect, line_width: Option<usize>) -> bool {
    format_once(stable, dialect, line_width).as_deref() == Some(stable)
}

/// Format one corpus query and run every check SQLClean's corpus test runs on it.
fn check_entry(
    entry: &Value,
    name: &str,
    departures: &[Departure],
    dialect: FormatDialect,
) -> EntryOutcome {
    let sql = entry["sql"].as_str().unwrap_or_default();
    let departure = departures.iter().find(|departure| departure.sql == sql);
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
        failures: checks
            .into_iter()
            .filter(|(passed, _)| !passed)
            .map(|(_, failure)| failure)
            .collect(),
    }
}

/// Whether `corpus` holds `sql` for `dialect`.
fn in_corpus(corpus: &[Value], dialect: &str, sql: &str) -> bool {
    corpus
        .iter()
        .any(|entry| entry["dialect"] == dialect && entry["sql"] == sql)
}

/// The formatted, refused and departing counts of the corpus queries named `name`, laid out as `dialect`, and every check that failed.
pub(crate) fn corpus_outcome(
    dialect: FormatDialect,
    name: &str,
    departures: &[Departure],
) -> CorpusOutcome {
    let corpus: Vec<Value> = serde_json::from_str(FORMAT_CORPUS).unwrap_or_default();
    let outcomes: Vec<EntryOutcome> = corpus
        .iter()
        .filter(|entry| entry["dialect"] == name)
        .map(|entry| check_entry(entry, name, departures, dialect))
        .collect();
    let formatted = outcomes.iter().filter(|outcome| outcome.formatted).count();
    CorpusOutcome {
        formatted,
        refused: outcomes.len() - formatted,
        departed: departures
            .iter()
            .filter(|departure| in_corpus(&corpus, name, departure.sql))
            .count(),
        failures: outcomes
            .into_iter()
            .flat_map(|outcome| outcome.failures)
            .collect(),
    }
}

/// `sql` laid out as `dialect` at `line_width`, or the reason it is refused.
pub(crate) fn format_outcome(
    sql: &str,
    dialect: FormatDialect,
    line_width: Option<usize>,
) -> Result<String, String> {
    let options = FormatOptions {
        line_width,
        ..FormatOptions::new(dialect)
    };
    format_sql(sql, &options)
        .map(|formatted| formatted.sql)
        .map_err(|refusal| refusal.reason)
}

/// The start of the reason the formatter refuses `sql` with as `dialect`; `None` when it is laid out.
pub(crate) fn refusal(sql: &str, dialect: FormatDialect) -> Option<String> {
    format_sql(sql, &FormatOptions::new(dialect))
        .err()
        .map(|refusal| refusal_start(&refusal.reason).to_string())
}

/// How long formatting `sql` in `dialect` takes, laid out or refused.
pub(crate) fn format_time(sql: &str, dialect: FormatDialect) -> std::time::Duration {
    let start = std::time::Instant::now();
    let _ = format_sql(sql, &FormatOptions::new(dialect));
    start.elapsed()
}

/// The failure constant `reason` starts with, or all of `reason` when it starts with none of them.
fn refusal_start(reason: &str) -> &str {
    [PARSE_FAILURE, DEPTH_FAILURE, TYPE_DEPTH_FAILURE]
        .into_iter()
        .find(|failure| reason.starts_with(failure))
        .unwrap_or(reason)
}
