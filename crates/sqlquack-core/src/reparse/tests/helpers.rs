use polyglot_sql::Expression;
use serde_json::Value;

use crate::parsing::main::parse_source::parse_source;
use crate::parsing::main::parse_target::parse_target;
use crate::parsing::models::SourceDialect;
use crate::reparse::main::check_duckdb_print::check_duckdb_print;
use crate::reparse::main::check_source_fidelity::check_source_fidelity;

fn single(mut statements: Vec<Expression>) -> Expression {
    assert_eq!(statements.len(), 1, "test SQL holds one statement");
    statements.remove(0)
}

fn path_of(report: &Value) -> Option<String> {
    report["difference"]["path"].as_str().map(str::to_owned)
}

pub(crate) fn fidelity_report(sql: &str) -> Value {
    let statement = single(parse_source(sql, SourceDialect::Snowflake).expect("test SQL parses"));
    serde_json::to_value(check_source_fidelity(&statement, SourceDialect::Snowflake))
        .expect("outcomes serialise")
}

pub(crate) fn fidelity_outcome(report: &Value) -> String {
    report["outcome"].as_str().unwrap_or_default().to_owned()
}

pub(crate) fn fidelity_path(report: &Value) -> Option<String> {
    path_of(report)
}

pub(crate) fn printer_report(sql: &str) -> Value {
    let intended = single(parse_target(sql).expect("test SQL parses"));
    serde_json::to_value(check_duckdb_print(&intended).err()).expect("failures serialise")
}

pub(crate) fn printer_failure(report: &Value) -> Option<&str> {
    report["failure"].as_str()
}

pub(crate) fn printer_path(report: &Value) -> Option<String> {
    path_of(report)
}
