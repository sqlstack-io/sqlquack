//! `compare_parse`: one statement parsed by both parsers.

use std::time::Instant;

use polyglot_sql::DialectType;
use sqlquack_syntax::{CallMode, Dialect, ParseOptions};
use sqlquack_syntax_snowflake::parse_statements_with;

use crate::comparison::_helpers::outcomes::{lossless_failure, rendered};
use crate::comparison::models::{Comparison, Outcome};

/// Parses `sql` with polyglot and with SQLQuack lowering calls as polyglot does, and compares.
pub fn compare_parse(sql: &str, dialect: Dialect) -> Comparison {
    let polyglot_dialect = match dialect {
        Dialect::Snowflake => DialectType::Snowflake,
        Dialect::DuckDB => DialectType::DuckDB,
        Dialect::PostgreSQL => DialectType::PostgreSQL,
        Dialect::SQLite => DialectType::SQLite,
        Dialect::BigQuery => DialectType::BigQuery,
        Dialect::ClickHouse => DialectType::ClickHouse,
        Dialect::Databricks => DialectType::Databricks,
        Dialect::SqlServer => DialectType::TSQL,
    };
    let started = Instant::now();
    let theirs = rendered(|| polyglot_sql::parse(sql, polyglot_dialect));
    let polyglot = started.elapsed();
    let started = Instant::now();
    let ours = rendered(|| {
        parse_statements_with(sql, dialect, &ParseOptions::default(), CallMode::Lowered)
    });
    let sqlquack = started.elapsed();
    let parsed = ours.starts_with("ok:");
    let outcome = if theirs != ours {
        Outcome::Differs {
            polyglot: theirs,
            sqlquack: ours,
        }
    } else if let Some(reason) = lossless_failure(sql, dialect, parsed) {
        Outcome::TokensLost { reason }
    } else if parsed {
        Outcome::SameTree
    } else {
        Outcome::SameError
    };
    Comparison {
        outcome,
        polyglot,
        sqlquack,
    }
}
