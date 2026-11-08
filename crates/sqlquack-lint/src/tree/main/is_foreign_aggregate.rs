//! `is_foreign_aggregate`: whether a call name is another dialect's aggregate the parsed dialect lacks.

use crate::lint::models::LintDialect;
use crate::tree::constants::{
    BIGQUERY_FOREIGN_AGGREGATE_CALLS, POSTGRES_FOREIGN_AGGREGATE_CALLS,
    SNOWFLAKE_FOREIGN_AGGREGATE_CALLS,
};

/// Whether `name` is another dialect's aggregate that `dialect` has none of, so its kind is unknown.
pub(crate) fn is_foreign_aggregate(name: &str, dialect: LintDialect) -> bool {
    let foreign: &[&str] = match dialect {
        LintDialect::Snowflake => &SNOWFLAKE_FOREIGN_AGGREGATE_CALLS,
        LintDialect::PostgreSQL => &POSTGRES_FOREIGN_AGGREGATE_CALLS,
        LintDialect::BigQuery => &BIGQUERY_FOREIGN_AGGREGATE_CALLS,
        LintDialect::DuckDB | LintDialect::SQLite => &[],
    };
    foreign
        .iter()
        .any(|aggregate| aggregate.eq_ignore_ascii_case(name))
}
