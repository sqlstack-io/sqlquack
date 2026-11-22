//! A located parse records where each set operation's keyword is written, in every parser.

use sqlquack_syntax::tokens::models::Span;
use sqlquack_syntax::{Dialect, Expression, ExpressionWalk};

use crate::parsing::main::parse_source::parse_source;
use crate::parsing::tests::test_types::OperatorSpanTestCase;

#[test]
fn given_set_operations_when_parsed_located_then_each_records_its_operator_keyword() {
    let test_cases = [
        OperatorSpanTestCase {
            description: "Snowflake, with MINUS, BY NAME and INTERSECT binding tighter",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1 AS a UNION ALL BY NAME SELECT 2 AS a MINUS SELECT 3 INTERSECT SELECT 4",
            expected_operators: &["UNION", "MINUS", "INTERSECT"],
        },
        OperatorSpanTestCase {
            description: "DuckDB through the shared parser, with a parenthesised branch",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT a FROM x UNION ALL (SELECT b FROM y EXCEPT SELECT c FROM z)",
            expected_operators: &["UNION", "EXCEPT"],
        },
        OperatorSpanTestCase {
            description: "SQLite through the shared parser, after a character of several bytes",
            parse: |sql| parse_source(sql, Dialect::SQLite).ok(),
            sql: "SELECT 'é' UNION ALL SELECT 'b'",
            expected_operators: &["UNION"],
        },
        #[cfg(feature = "postgres")]
        OperatorSpanTestCase {
            description: "PostgreSQL",
            parse: |sql| {
                sqlquack_syntax_postgres::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT 'é' UNION DISTINCT SELECT 'b' intersect SELECT 'c'",
            expected_operators: &["UNION", "intersect"],
        },
        #[cfg(feature = "bigquery")]
        OperatorSpanTestCase {
            description: "BigQuery, after a star EXCEPT",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * EXCEPT (b) FROM x UNION ALL SELECT a FROM y",
            expected_operators: &["UNION"],
        },
        #[cfg(feature = "bigquery")]
        OperatorSpanTestCase {
            description: "a BigQuery pipe set operation",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "FROM x |> UNION ALL (SELECT a FROM y)",
            expected_operators: &["UNION"],
        },
        #[cfg(feature = "databricks")]
        OperatorSpanTestCase {
            description: "Databricks",
            parse: |sql| {
                sqlquack_syntax_databricks::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT a FROM x MINUS SELECT a FROM y UNION ALL SELECT a FROM z",
            expected_operators: &["MINUS", "UNION"],
        },
        #[cfg(feature = "databricks")]
        OperatorSpanTestCase {
            description: "a Databricks pipe set operation",
            parse: |sql| {
                sqlquack_syntax_databricks::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "FROM x |> UNION ALL SELECT a FROM y",
            expected_operators: &["UNION"],
        },
        #[cfg(feature = "duckdb")]
        OperatorSpanTestCase {
            description: "DuckDB's own parser",
            parse: |sql| {
                sqlquack_syntax_duckdb::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT 1 AS a UNION BY NAME SELECT 2 AS a INTERSECT SELECT 3 AS a",
            expected_operators: &["UNION", "INTERSECT"],
        },
        #[cfg(feature = "sqlite")]
        OperatorSpanTestCase {
            description: "SQLite's own parser",
            parse: |sql| {
                sqlquack_syntax_sqlite::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT 1 UNION ALL SELECT 2 EXCEPT SELECT 3",
            expected_operators: &["UNION", "EXCEPT"],
        },
        #[cfg(feature = "clickhouse")]
        OperatorSpanTestCase {
            description: "ClickHouse",
            parse: |sql| {
                sqlquack_syntax_clickhouse::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT 1 UNION ALL SELECT 2 EXCEPT SELECT 3",
            expected_operators: &["UNION", "EXCEPT"],
        },
        #[cfg(feature = "sqlserver")]
        OperatorSpanTestCase {
            description: "SQL Server",
            parse: |sql| {
                sqlquack_syntax_sqlserver::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT 1 AS a UNION ALL SELECT 2 INTERSECT SELECT 3",
            expected_operators: &["UNION", "INTERSECT"],
        },
    ];

    for test_case in test_cases {
        let statements: Vec<Expression> = (test_case.parse)(test_case.sql).unwrap_or_default();
        let mut spans: Vec<Option<Span>> = statements
            .iter()
            .flat_map(|statement| statement.dfs())
            .filter_map(|node| node.set_operator_span())
            .map(|operator| operator.0)
            .collect();
        spans.sort_by_key(|span| span.map(|span| span.start));
        let written: Vec<String> = spans
            .into_iter()
            .map(|span| {
                span.map_or("<no range>".to_owned(), |span| {
                    test_case
                        .sql
                        .chars()
                        .skip(span.start)
                        .take(span.end - span.start)
                        .collect()
                })
            })
            .collect();
        assert_eq!(
            written, test_case.expected_operators,
            "{}",
            test_case.description
        );
    }
}
