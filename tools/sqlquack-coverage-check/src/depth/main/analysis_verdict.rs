//! `analysis_verdict`: a parser's verdict on SQL it then analyses, lineage and findings.

use crate::check::models::LocalVerdict;
use crate::depth::_helpers::analysed::{
    duckdb_analysis, postgres_analysis, snowflake_analysis, sqlite_analysis,
};
use crate::formats::models::LocalParser;

/// The parse-and-analyse verdict of `parser`, or none when it has no analysis profile.
pub fn analysis_verdict(parser: LocalParser) -> Option<fn(&str) -> LocalVerdict> {
    match parser {
        LocalParser::Snowflake => Some(snowflake_analysis),
        LocalParser::DuckDb => Some(duckdb_analysis),
        LocalParser::Postgres => Some(postgres_analysis),
        LocalParser::Sqlite => Some(sqlite_analysis),
        LocalParser::BigQuery
        | LocalParser::Databricks
        | LocalParser::SqlServer
        | LocalParser::None => None,
    }
}
