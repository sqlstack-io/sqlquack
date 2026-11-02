//! Whole-corpus parse passes.

use std::time::{Duration, Instant};

use polyglot_sql::DialectType;
use sqlquack_syntax::{Dialect, ParseOptions, parse_statements};

use crate::corpora::models::CorpusEntry;

/// polyglot's time to parse every entry once.
pub(crate) fn polyglot_pass(entries: &[CorpusEntry]) -> Duration {
    let started = Instant::now();
    for entry in entries {
        let dialect = match entry.dialect {
            Dialect::Snowflake => DialectType::Snowflake,
            Dialect::DuckDB => DialectType::DuckDB,
            Dialect::PostgreSQL => DialectType::PostgreSQL,
        };
        let _ = std::hint::black_box(polyglot_sql::parse(&entry.sql, dialect));
    }
    started.elapsed()
}

/// SQLQuack's time to parse every entry once.
pub(crate) fn sqlquack_pass(entries: &[CorpusEntry]) -> Duration {
    let started = Instant::now();
    let options = ParseOptions::default();
    for entry in entries {
        let _ = std::hint::black_box(parse_statements(&entry.sql, entry.dialect, &options));
    }
    started.elapsed()
}

/// The middle time, or zero when there are none.
pub(crate) fn median(mut times: Vec<Duration>) -> Duration {
    times.sort();
    times.get(times.len() / 2).copied().unwrap_or_default()
}
