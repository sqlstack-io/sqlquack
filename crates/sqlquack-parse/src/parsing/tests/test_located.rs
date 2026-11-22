//! A located parse records where the keywords and separators of queries are written.

use sqlquack_syntax::expressions::queries::models::Select;
use sqlquack_syntax::tokens::models::{SourceSpan, Span};
use sqlquack_syntax::{Dialect, Expression, ExpressionWalk};

use crate::parsing::main::parse_source::parse_source;
use crate::parsing::tests::test_types::{JoinSpanTestCase, OperatorSpanTestCase};

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

#[test]
fn given_joins_when_parsed_located_then_each_records_its_keywords_and_commas() {
    let test_cases = [
        JoinSpanTestCase {
            description: "a comma join's commas are between the items",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT * FROM a, b, c",
            expected_joins: &[],
            expected_commas: &[15, 18],
        },
        JoinSpanTestCase {
            description: "a plain join and an outer join",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT * FROM a JOIN b ON a.x = b.x LEFT OUTER join c on c.x = a.x",
            expected_joins: &[["JOIN", "JOIN", "ON"], ["LEFT", "join", "on"]],
            expected_commas: &[],
        },
        JoinSpanTestCase {
            description: "a directed cross join",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT * FROM a CROSS DIRECTED JOIN b",
            expected_joins: &[["CROSS", "JOIN", ""]],
            expected_commas: &[],
        },
        JoinSpanTestCase {
            description: "an ASOF join without ON",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT * FROM t ASOF JOIN q MATCH_CONDITION (t.ts >= q.ts)",
            expected_joins: &[["ASOF", "JOIN", ""]],
            expected_commas: &[],
        },
        JoinSpanTestCase {
            description: "a comma after a join",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT * FROM a JOIN b ON a.x = b.x, c",
            expected_joins: &[["JOIN", "JOIN", "ON"], [",", ",", ""]],
            expected_commas: &[],
        },
        JoinSpanTestCase {
            description: "conditions written after a chain of joins",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT * FROM a JOIN b JOIN c ON b.x = c.x on a.x = b.x",
            expected_joins: &[["JOIN", "JOIN", "on"], ["JOIN", "JOIN", "ON"]],
            expected_commas: &[],
        },
        #[cfg(feature = "postgres")]
        JoinSpanTestCase {
            description: "PostgreSQL joins",
            parse: |sql| {
                sqlquack_syntax_postgres::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a LEFT OUTER JOIN b ON a.x = b.x CROSS JOIN c",
            expected_joins: &[["LEFT", "JOIN", "ON"], ["CROSS", "JOIN", ""]],
            expected_commas: &[],
        },
        #[cfg(feature = "postgres")]
        JoinSpanTestCase {
            description: "PostgreSQL commas",
            parse: |sql| {
                sqlquack_syntax_postgres::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a, b, c",
            expected_joins: &[],
            expected_commas: &[15, 18],
        },
        #[cfg(feature = "bigquery")]
        JoinSpanTestCase {
            description: "BigQuery joins",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a LEFT OUTER JOIN b ON a.x = b.x CROSS JOIN c",
            expected_joins: &[["LEFT", "JOIN", "ON"], ["CROSS", "JOIN", ""]],
            expected_commas: &[],
        },
        #[cfg(feature = "bigquery")]
        JoinSpanTestCase {
            description: "BigQuery commas",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a, b, c",
            expected_joins: &[],
            expected_commas: &[15, 18],
        },
        #[cfg(feature = "databricks")]
        JoinSpanTestCase {
            description: "Databricks joins",
            parse: |sql| {
                sqlquack_syntax_databricks::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a LEFT OUTER JOIN b ON a.x = b.x CROSS JOIN c",
            expected_joins: &[["LEFT", "JOIN", "ON"], ["CROSS", "JOIN", ""]],
            expected_commas: &[],
        },
        #[cfg(feature = "databricks")]
        JoinSpanTestCase {
            description: "Databricks commas",
            parse: |sql| {
                sqlquack_syntax_databricks::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a, b, c",
            expected_joins: &[],
            expected_commas: &[15, 18],
        },
        #[cfg(feature = "duckdb")]
        JoinSpanTestCase {
            description: "DuckDB's own parser joins",
            parse: |sql| {
                sqlquack_syntax_duckdb::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a LEFT OUTER JOIN b ON a.x = b.x CROSS JOIN c",
            expected_joins: &[["LEFT", "JOIN", "ON"], ["CROSS", "JOIN", ""]],
            expected_commas: &[],
        },
        #[cfg(feature = "duckdb")]
        JoinSpanTestCase {
            description: "DuckDB's own parser commas",
            parse: |sql| {
                sqlquack_syntax_duckdb::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a, b, c",
            expected_joins: &[],
            expected_commas: &[15, 18],
        },
        #[cfg(feature = "sqlite")]
        JoinSpanTestCase {
            description: "SQLite's own parser joins",
            parse: |sql| {
                sqlquack_syntax_sqlite::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a LEFT OUTER JOIN b ON a.x = b.x CROSS JOIN c",
            expected_joins: &[["LEFT", "JOIN", "ON"], ["CROSS", "JOIN", ""]],
            expected_commas: &[],
        },
        #[cfg(feature = "sqlite")]
        JoinSpanTestCase {
            description: "SQLite's own parser commas",
            parse: |sql| {
                sqlquack_syntax_sqlite::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a, b, c",
            expected_joins: &[],
            expected_commas: &[15, 18],
        },
        #[cfg(feature = "clickhouse")]
        JoinSpanTestCase {
            description: "ClickHouse joins",
            parse: |sql| {
                sqlquack_syntax_clickhouse::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a LEFT OUTER JOIN b ON a.x = b.x CROSS JOIN c",
            expected_joins: &[["LEFT", "JOIN", "ON"], ["CROSS", "JOIN", ""]],
            expected_commas: &[],
        },
        #[cfg(feature = "clickhouse")]
        JoinSpanTestCase {
            description: "ClickHouse commas",
            parse: |sql| {
                sqlquack_syntax_clickhouse::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a, b, c",
            expected_joins: &[],
            expected_commas: &[15, 18],
        },
        #[cfg(feature = "sqlserver")]
        JoinSpanTestCase {
            description: "SQL Server joins",
            parse: |sql| {
                sqlquack_syntax_sqlserver::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a LEFT OUTER JOIN b ON a.x = b.x CROSS JOIN c",
            expected_joins: &[["LEFT", "JOIN", "ON"], ["CROSS", "JOIN", ""]],
            expected_commas: &[],
        },
        #[cfg(feature = "sqlserver")]
        JoinSpanTestCase {
            description: "SQL Server commas",
            parse: |sql| {
                sqlquack_syntax_sqlserver::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a, b, c",
            expected_joins: &[],
            expected_commas: &[15, 18],
        },
        #[cfg(feature = "bigquery")]
        JoinSpanTestCase {
            description: "a BigQuery comma after a join",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a JOIN b ON a.x = b.x, c",
            expected_joins: &[["JOIN", "JOIN", "ON"], [",", ",", ""]],
            expected_commas: &[],
        },
        #[cfg(feature = "bigquery")]
        JoinSpanTestCase {
            description: "BigQuery conditions written after their joins",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a JOIN b JOIN c ON b.x = c.x on a.x = b.x",
            expected_joins: &[["JOIN", "JOIN", "on"], ["JOIN", "JOIN", "ON"]],
            expected_commas: &[],
        },
        #[cfg(feature = "sqlite")]
        JoinSpanTestCase {
            description: "a SQLite comma after a join",
            parse: |sql| {
                sqlquack_syntax_sqlite::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a JOIN b ON a.x = b.x, c",
            expected_joins: &[["JOIN", "JOIN", "ON"], [",", ",", ""]],
            expected_commas: &[],
        },
        #[cfg(feature = "duckdb")]
        JoinSpanTestCase {
            description: "a trailing DuckDB comma separates nothing",
            parse: |sql| {
                sqlquack_syntax_duckdb::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a, b,",
            expected_joins: &[],
            expected_commas: &[15],
        },
        #[cfg(feature = "duckdb")]
        JoinSpanTestCase {
            description: "a DuckDB ASOF join",
            parse: |sql| {
                sqlquack_syntax_duckdb::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a ASOF JOIN b ON a.ts >= b.ts",
            expected_joins: &[["ASOF", "JOIN", "ON"]],
            expected_commas: &[],
        },
        #[cfg(feature = "clickhouse")]
        JoinSpanTestCase {
            description: "a ClickHouse comma after a join",
            parse: |sql| {
                sqlquack_syntax_clickhouse::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a JOIN b ON a.x = b.x, c",
            expected_joins: &[["JOIN", "JOIN", "ON"], [",", ",", ""]],
            expected_commas: &[],
        },
        #[cfg(feature = "sqlserver")]
        JoinSpanTestCase {
            description: "a SQL Server APPLY",
            parse: |sql| {
                sqlquack_syntax_sqlserver::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * FROM a CROSS APPLY f(a.x) AS b",
            expected_joins: &[["CROSS", "APPLY", ""]],
            expected_commas: &[],
        },
    ];

    for test_case in test_cases {
        let statements: Vec<Expression> = (test_case.parse)(test_case.sql).unwrap_or_default();
        let selects: Vec<&Select> = statements
            .iter()
            .flat_map(|statement| statement.dfs())
            .filter_map(Expression::as_select)
            .collect();
        let text = |span: SourceSpan| -> String {
            span.0.map_or(String::new(), |span| {
                test_case
                    .sql
                    .chars()
                    .skip(span.start)
                    .take(span.end - span.start)
                    .collect()
            })
        };
        let joins: Vec<[String; 3]> = statements
            .iter()
            .flat_map(|statement| statement.dfs())
            .flat_map(Expression::own_joins)
            .map(|join| {
                [
                    text(join.kind_span),
                    text(join.keyword_span),
                    text(join.on_span),
                ]
            })
            .collect();
        let commas: Vec<usize> = selects
            .iter()
            .filter_map(|select| select.from.as_ref())
            .flat_map(|from| &from.comma_spans.0)
            .map(|comma| comma.start)
            .collect();
        let expected_joins: Vec<[String; 3]> = test_case
            .expected_joins
            .iter()
            .map(|join| join.map(str::to_owned))
            .collect();
        assert_eq!(
            (joins, commas),
            (expected_joins, test_case.expected_commas.to_vec()),
            "{}",
            test_case.description
        );
    }
}
