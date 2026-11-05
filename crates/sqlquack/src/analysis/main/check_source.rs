//! `check_source`: what is wrong with some Snowflake SQL before any schema or conversion.

use sqlquack_core::diagnostics::main::diagnose::diagnose;
use sqlquack_core::diagnostics::models::Diagnostic;
use sqlquack_core::parsing::main::parse_source::parse_source;
use sqlquack_core::parsing::models::{SourceDialect, SourceLines};
use sqlquack_core::refusals::main::refuse_unparsed::refuse_unparsed;
use sqlquack_core::refusals::main::snowflake_rejections::snowflake_rejections;

/// The parse failure, or every construct Snowflake would reject; empty when the SQL is sound.
pub fn check_source(sql: &str) -> Result<Vec<Diagnostic>, String> {
    let lines = SourceLines::new(sql);
    let refusals = match parse_source(sql, SourceDialect::Snowflake) {
        Ok(statements) if statements.is_empty() => {
            return Err("the input holds no SQL statement".to_owned());
        }
        Ok(statements) => statements
            .iter()
            .flat_map(|statement| snowflake_rejections(statement, (sql, &lines)))
            .collect(),
        Err(failure) => vec![refuse_unparsed(&failure, &lines)],
    };
    Ok(diagnose(&refusals, sql))
}
