//! A located parse records where clauses, list items, operators and arguments are written.

use sqlquack_syntax::{Dialect, Expression};

use crate::parsing::tests::helpers::{parse_as_written, written_positions};
use crate::parsing::tests::test_types::WrittenPositionsTestCase;

#[test]
fn given_queries_and_writes_when_parsed_located_then_clauses_items_and_operators_know_their_positions()
 {
    let test_cases = [
        WrittenPositionsTestCase {
            description: "Snowflake, a query",
            parse: |sql| parse_as_written(sql, Dialect::Snowflake),
            sql: "WITH c AS (SELECT DISTINCT x, 2 FROM t AS u WHERE x NOT IN (1) GROUP BY x), d AS (SELECT 3 AS y) SELECT COUNT(1), CASE y WHEN 0 THEN 1 END FROM d WHERE y BETWEEN 3 AND 4 ORDER BY 1",
            expected_positions: &[
                "alias AS u",
                "argument 1",
                "comma ,",
                "from FROM",
                "from FROM",
                "group GROUP",
                "head 3",
                "head COUNT",
                "head DISTINCT",
                "item 2",
                "item 3 AS y",
                "item CASE y WHEN 0 THEN 1 END",
                "item COUNT(1)",
                "item x",
                "operation BETWEEN 3 AND 4",
                "operation NOT IN (1)",
                "order ORDER",
                "select SELECT",
                "select SELECT",
                "select SELECT",
                "when WHEN 0",
                "with WITH",
            ],
        },
        WrittenPositionsTestCase {
            description: "Snowflake, a DELETE",
            parse: |sql| parse_as_written(sql, Dialect::Snowflake),
            sql: "DELETE FROM t WHERE x / 2 = 1",
            expected_positions: &["delete DELETE", "operation / 2", "operation = 1"],
        },
        WrittenPositionsTestCase {
            description: "DuckDB through the shared parser, a query",
            parse: |sql| parse_as_written(sql, Dialect::DuckDB),
            sql: "WITH c AS (SELECT DISTINCT x, 2 FROM t AS u WHERE x NOT IN (1) GROUP BY x), d AS (SELECT 3 AS y) SELECT COUNT(1), CASE y WHEN 0 THEN 1 END FROM d WHERE y BETWEEN 3 AND 4 ORDER BY 1",
            expected_positions: &[
                "alias AS u",
                "argument 1",
                "comma ,",
                "from FROM",
                "from FROM",
                "group GROUP",
                "head 3",
                "head COUNT",
                "head DISTINCT",
                "item 2",
                "item 3 AS y",
                "item CASE y WHEN 0 THEN 1 END",
                "item COUNT(1)",
                "item x",
                "operation BETWEEN 3 AND 4",
                "operation NOT IN (1)",
                "order ORDER",
                "select SELECT",
                "select SELECT",
                "select SELECT",
                "when WHEN 0",
                "with WITH",
            ],
        },
        WrittenPositionsTestCase {
            description: "DuckDB through the shared parser, a DELETE",
            parse: |sql| parse_as_written(sql, Dialect::DuckDB),
            sql: "DELETE FROM t WHERE x / 2 = 1",
            expected_positions: &["delete DELETE", "operation / 2", "operation = 1"],
        },
        WrittenPositionsTestCase {
            description: "SQLite through the shared parser, a query",
            parse: |sql| parse_as_written(sql, Dialect::SQLite),
            sql: "WITH c AS (SELECT DISTINCT x, 2 FROM t AS u WHERE x NOT IN (1) GROUP BY x), d AS (SELECT 3 AS y) SELECT COUNT(1), CASE y WHEN 0 THEN 1 END FROM d WHERE y BETWEEN 3 AND 4 ORDER BY 1",
            expected_positions: &[
                "alias AS u",
                "argument 1",
                "comma ,",
                "from FROM",
                "from FROM",
                "group GROUP",
                "head 3",
                "head COUNT",
                "head DISTINCT",
                "item 2",
                "item 3 AS y",
                "item CASE y WHEN 0 THEN 1 END",
                "item COUNT(1)",
                "item x",
                "operation BETWEEN 3 AND 4",
                "operation NOT IN (1)",
                "order ORDER",
                "select SELECT",
                "select SELECT",
                "select SELECT",
                "when WHEN 0",
                "with WITH",
            ],
        },
        WrittenPositionsTestCase {
            description: "SQLite through the shared parser, a DELETE",
            parse: |sql| parse_as_written(sql, Dialect::SQLite),
            sql: "DELETE FROM t WHERE x / 2 = 1",
            expected_positions: &["delete DELETE", "operation / 2", "operation = 1"],
        },
        #[cfg(feature = "postgres")]
        WrittenPositionsTestCase {
            description: "PostgreSQL's own parser, a query",
            parse: |sql| {
                sqlquack_syntax_postgres::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "WITH c AS (SELECT DISTINCT x, 2 FROM t AS u WHERE x NOT IN (1) GROUP BY x), d AS (SELECT 3 AS y) SELECT COUNT(1), CASE y WHEN 0 THEN 1 END FROM d WHERE y BETWEEN 3 AND 4 ORDER BY 1",
            expected_positions: &[
                "alias AS u",
                "argument 1",
                "comma ,",
                "from FROM",
                "from FROM",
                "group GROUP",
                "head 3",
                "head COUNT",
                "head DISTINCT",
                "item 2",
                "item 3 AS y",
                "item CASE y WHEN 0 THEN 1 END",
                "item COUNT(1)",
                "item x",
                "operation BETWEEN 3 AND 4",
                "operation NOT IN (1)",
                "order ORDER",
                "select SELECT",
                "select SELECT",
                "select SELECT",
                "when WHEN 0",
                "with WITH",
            ],
        },
        #[cfg(feature = "postgres")]
        WrittenPositionsTestCase {
            description: "PostgreSQL's own parser, a DELETE",
            parse: |sql| {
                sqlquack_syntax_postgres::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "DELETE FROM t WHERE x / 2 = 1",
            expected_positions: &["delete DELETE", "operation / 2", "operation = 1"],
        },
        #[cfg(feature = "postgres")]
        WrittenPositionsTestCase {
            description: "PostgreSQL's own parser, a MERGE after a WITH and a SQL/JSON aggregate",
            parse: |sql| {
                sqlquack_syntax_postgres::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "WITH c AS (SELECT JSON_ARRAYAGG(x) AS v) MERGE INTO m USING c ON true WHEN MATCHED THEN DELETE",
            expected_positions: &[
                "call JSON_ARRAYAGG(x)",
                "head JSON_ARRAYAGG",
                "item JSON_ARRAYAGG(x) AS v",
                "select SELECT",
                "with WITH",
            ],
        },
        #[cfg(feature = "bigquery")]
        WrittenPositionsTestCase {
            description: "BigQuery's own parser, a query",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "WITH c AS (SELECT DISTINCT x, 2 FROM t AS u WHERE x NOT IN (1) GROUP BY x), d AS (SELECT 3 AS y) SELECT COUNT(1), CASE y WHEN 0 THEN 1 END FROM d WHERE y BETWEEN 3 AND 4 ORDER BY 1",
            expected_positions: &[
                "alias AS u",
                "argument 1",
                "comma ,",
                "from FROM",
                "from FROM",
                "group GROUP",
                "head 3",
                "head COUNT",
                "head DISTINCT",
                "item 2",
                "item 3 AS y",
                "item CASE y WHEN 0 THEN 1 END",
                "item COUNT(1)",
                "item x",
                "operation BETWEEN 3 AND 4",
                "operation NOT IN (1)",
                "order ORDER",
                "select SELECT",
                "select SELECT",
                "select SELECT",
                "when WHEN 0",
                "with WITH",
            ],
        },
        #[cfg(feature = "bigquery")]
        WrittenPositionsTestCase {
            description: "BigQuery's own parser, a DELETE",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "DELETE FROM t WHERE x / 2 = 1",
            expected_positions: &["delete DELETE", "operation / 2", "operation = 1"],
        },
        #[cfg(feature = "databricks")]
        WrittenPositionsTestCase {
            description: "Databricks's own parser, a query",
            parse: |sql| {
                sqlquack_syntax_databricks::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "WITH c AS (SELECT DISTINCT x, 2 FROM t AS u WHERE x NOT IN (1) GROUP BY x), d AS (SELECT 3 AS y) SELECT COUNT(1), CASE y WHEN 0 THEN 1 END FROM d WHERE y BETWEEN 3 AND 4 ORDER BY 1",
            expected_positions: &[
                "alias AS u",
                "argument 1",
                "comma ,",
                "from FROM",
                "from FROM",
                "group GROUP",
                "head 3",
                "head COUNT",
                "head DISTINCT",
                "item 2",
                "item 3 AS y",
                "item CASE y WHEN 0 THEN 1 END",
                "item COUNT(1)",
                "item x",
                "operation BETWEEN 3 AND 4",
                "operation NOT IN (1)",
                "order ORDER",
                "select SELECT",
                "select SELECT",
                "select SELECT",
                "when WHEN 0",
                "with WITH",
            ],
        },
        #[cfg(feature = "databricks")]
        WrittenPositionsTestCase {
            description: "Databricks's own parser, a DELETE",
            parse: |sql| {
                sqlquack_syntax_databricks::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "DELETE FROM t WHERE x / 2 = 1",
            expected_positions: &["delete DELETE", "operation / 2", "operation = 1"],
        },
        #[cfg(feature = "duckdb")]
        WrittenPositionsTestCase {
            description: "DuckDB's own parser, a query",
            parse: |sql| {
                sqlquack_syntax_duckdb::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "WITH c AS (SELECT DISTINCT x, 2 FROM t AS u WHERE x NOT IN (1) GROUP BY x), d AS (SELECT 3 AS y) SELECT COUNT(1), CASE y WHEN 0 THEN 1 END FROM d WHERE y BETWEEN 3 AND 4 ORDER BY 1",
            expected_positions: &[
                "alias AS u",
                "comma ,",
                "from FROM",
                "from FROM",
                "group GROUP",
                "head 3",
                "head COUNT",
                "head DISTINCT",
                "item 2",
                "item 3 AS y",
                "item CASE y WHEN 0 THEN 1 END",
                "item COUNT(1)",
                "item x",
                "operation BETWEEN 3 AND 4",
                "operation NOT IN (1)",
                "order ORDER",
                "select SELECT",
                "select SELECT",
                "select SELECT",
                "when WHEN 0",
                "with WITH",
            ],
        },
        #[cfg(feature = "duckdb")]
        WrittenPositionsTestCase {
            description: "DuckDB's own parser, a DELETE",
            parse: |sql| {
                sqlquack_syntax_duckdb::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "DELETE FROM t WHERE x / 2 = 1",
            expected_positions: &["delete DELETE", "operation / 2", "operation = 1"],
        },
        #[cfg(feature = "sqlite")]
        WrittenPositionsTestCase {
            description: "SQLite's own parser, a query",
            parse: |sql| {
                sqlquack_syntax_sqlite::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "WITH c AS (SELECT DISTINCT x, 2 FROM t AS u WHERE x NOT IN (1) GROUP BY x), d AS (SELECT 3 AS y) SELECT COUNT(1), CASE y WHEN 0 THEN 1 END FROM d WHERE y BETWEEN 3 AND 4 ORDER BY 1",
            expected_positions: &[
                "alias AS u",
                "comma ,",
                "from FROM",
                "from FROM",
                "group GROUP",
                "head 3",
                "head COUNT",
                "head DISTINCT",
                "item 2",
                "item 3 AS y",
                "item CASE y WHEN 0 THEN 1 END",
                "item COUNT(1)",
                "item x",
                "operation BETWEEN 3 AND 4",
                "operation NOT IN (1)",
                "order ORDER",
                "select SELECT",
                "select SELECT",
                "select SELECT",
                "when WHEN 0",
                "with WITH",
            ],
        },
        #[cfg(feature = "sqlserver")]
        WrittenPositionsTestCase {
            description: "SQL Server's own parser, a query",
            parse: |sql| {
                sqlquack_syntax_sqlserver::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "WITH c AS (SELECT DISTINCT x, 2 FROM t AS u WHERE x NOT IN (1) GROUP BY x), d AS (SELECT 3 AS y) SELECT COUNT(1), CASE y WHEN 0 THEN 1 END FROM d WHERE y BETWEEN 3 AND 4 ORDER BY 1",
            expected_positions: &[
                "alias AS u",
                "comma ,",
                "from FROM",
                "from FROM",
                "group GROUP",
                "head 3",
                "head COUNT",
                "head DISTINCT",
                "item 2",
                "item 3 AS y",
                "item CASE y WHEN 0 THEN 1 END",
                "item COUNT(1)",
                "item x",
                "operation BETWEEN 3 AND 4",
                "operation NOT IN (1)",
                "order ORDER",
                "select SELECT",
                "select SELECT",
                "select SELECT",
                "when WHEN 0",
                "with WITH",
            ],
        },
        #[cfg(feature = "sqlserver")]
        WrittenPositionsTestCase {
            description: "SQL Server's own parser, a DELETE",
            parse: |sql| {
                sqlquack_syntax_sqlserver::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "DELETE FROM t WHERE x / 2 = 1",
            expected_positions: &["delete DELETE", "operation / 2", "operation = 1"],
        },
        #[cfg(feature = "clickhouse")]
        WrittenPositionsTestCase {
            description: "ClickHouse's own parser, a query",
            parse: |sql| {
                sqlquack_syntax_clickhouse::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "WITH c AS (SELECT DISTINCT x, 2 FROM t AS u WHERE x NOT IN (1) GROUP BY x), d AS (SELECT 3 AS y) SELECT COUNT(1), CASE y WHEN 0 THEN 1 END FROM d WHERE y BETWEEN 3 AND 4 ORDER BY 1",
            expected_positions: &[
                "alias AS u",
                "comma ,",
                "from FROM",
                "from FROM",
                "group GROUP",
                "head 3",
                "head COUNT",
                "head DISTINCT",
                "item 2",
                "item 3 AS y",
                "item CASE y WHEN 0 THEN 1 END",
                "item COUNT(1)",
                "item x",
                "operation BETWEEN 3 AND 4",
                "operation NOT IN (1)",
                "order ORDER",
                "select SELECT",
                "select SELECT",
                "select SELECT",
                "when WHEN 0",
                "with WITH",
            ],
        },
        #[cfg(feature = "clickhouse")]
        WrittenPositionsTestCase {
            description: "ClickHouse's own parser, a DELETE",
            parse: |sql| {
                sqlquack_syntax_clickhouse::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "DELETE FROM t WHERE x / 2 = 1",
            expected_positions: &["delete DELETE", "operation / 2", "operation = 1"],
        },
    ];
    for test_case in test_cases {
        let statements: Vec<Expression> = (test_case.parse)(test_case.sql).unwrap_or_default();
        assert_eq!(
            written_positions(&statements, test_case.sql),
            test_case.expected_positions,
            "{}",
            test_case.description
        );
    }
}
