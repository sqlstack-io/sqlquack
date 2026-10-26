//! Strict and tolerant parsing.

use sqlquack_syntax::{parse_strict, parse_tolerant, Dialect, ParseOptions, StrictFailure};

#[test]
fn strict_parsing_refuses_any_error() {
    let options = ParseOptions::default();
    assert_eq!(
        parse_strict("SELECT 1; SELECT 2", Dialect::Snowflake, &options)
            .expect("valid SQL parses")
            .len(),
        2
    );
    assert!(matches!(
        parse_strict("SELECT 1; SELECT FROM WHERE", Dialect::Snowflake, &options),
        Err(StrictFailure::Parse(_))
    ));
}

#[test]
fn tolerant_parsing_keeps_every_statement_range() {
    let sql = "SELECT 1;\nSELECT FROM WHERE;\n-- note\nSELECT ';' AS s; ";
    let parse = parse_tolerant(sql, Dialect::Snowflake, &ParseOptions::default());
    let joined: String = parse.tokens.iter().map(|token| token.text(sql)).collect();
    assert_eq!(joined, sql);
    let ranges: Vec<(&str, bool)> = parse
        .statements
        .iter()
        .map(|statement| {
            (
                &sql[statement.start..statement.end],
                statement.parsed.is_ok(),
            )
        })
        .collect();
    assert_eq!(
        ranges,
        vec![
            ("SELECT 1", true),
            ("SELECT FROM WHERE", false),
            ("SELECT ';' AS s", true),
        ]
    );
}

#[test]
fn tolerant_parsing_keeps_untokenizable_input() {
    let sql = "  SELECT 'unterminated  ";
    let parse = parse_tolerant(sql, Dialect::DuckDB, &ParseOptions::default());
    assert!(parse.tokens.is_empty());
    assert!(parse.token_failure.is_some());
    assert_eq!(parse.statements.len(), 1);
    assert_eq!(
        &sql[parse.statements[0].start..parse.statements[0].end],
        "SELECT 'unterminated"
    );
    assert!(parse.statements[0].parsed.is_err());
}

#[test]
fn tolerant_parsing_splits_only_top_level_semicolons() {
    let sql = "SELECT f(1; 2); SELECT 3";
    let parse = parse_tolerant(sql, Dialect::Snowflake, &ParseOptions::default());
    let ranges: Vec<&str> = parse
        .statements
        .iter()
        .map(|statement| &sql[statement.start..statement.end])
        .collect();
    assert_eq!(ranges, vec!["SELECT f(1; 2)", "SELECT 3"]);
}

#[test]
fn tolerant_parsing_ends_on_a_token_no_option_value_reads() {
    let sql = "ALTER TABLE l SET STAGE_FILE_FORMAT(T=(=";
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for dialect in [Dialect::Snowflake, Dialect::DuckDB] {
            let _ = parse_tolerant(sql, dialect, &ParseOptions::default());
        }
        let _ = sender.send(());
    });
    assert!(
        receiver
            .recv_timeout(std::time::Duration::from_secs(30))
            .is_ok(),
        "tolerant parsing did not finish"
    );
}

#[test]
fn tolerant_parsing_ends_on_unterminated_option_lists() {
    for sql in [
        "CREATE TABLE c (pk BIGINT GENERATED ALWAYS AS IDENTITY (START WITH 10",
        "CREATE TABLE t (a INT) WITH (SYSTEM_VERSIONING = ON (HISTORY_TABLE = dbo.h",
        "ALTER TABLE t SET (DATA_DELETION = ON (FILTER_COLUMN = a",
    ] {
        for dialect in [Dialect::Snowflake, Dialect::DuckDB] {
            let parse = parse_tolerant(sql, dialect, &ParseOptions::default());
            let joined: String = parse.tokens.iter().map(|token| token.text(sql)).collect();
            assert_eq!(joined, sql, "{dialect:?}: {sql}");
        }
    }
}
