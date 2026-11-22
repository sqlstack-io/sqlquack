//! A located parse records where the keywords and separators of queries are written.

use sqlquack_syntax::expressions::queries::models::Select;
use sqlquack_syntax::tokens::models::{SourceSpan, Span};
use sqlquack_syntax::{Dialect, Expression, ExpressionWalk};

use crate::parsing::main::parse_source::parse_source;
use crate::parsing::tests::helpers::{case_keywords, parse_as_written};
use crate::parsing::tests::test_types::{
    CaseKeywordsTestCase, DistinctSpanTestCase, JoinSpanTestCase, OperatorSpanTestCase,
    RowLimitSpanTestCase, WrittenCallTestCase,
};

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

#[test]
fn given_select_distinct_when_parsed_located_then_it_records_the_distinct_keyword() {
    let test_cases = [
        DistinctSpanTestCase {
            description: "the shared parser, in and around a subquery",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT DISTINCT a FROM (SELECT DISTINCT a FROM t) AS s",
            expected_starts: &[7, 31],
        },
        DistinctSpanTestCase {
            description: "only a SELECT's own DISTINCT",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT COUNT(DISTINCT a) FROM t",
            expected_starts: &[],
        },
        DistinctSpanTestCase {
            description: "DISTINCT ON",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT DISTINCT ON (a) a, b FROM t",
            expected_starts: &[7],
        },
        #[cfg(feature = "postgres")]
        DistinctSpanTestCase {
            description: "postgres",
            parse: |sql| {
                sqlquack_syntax_postgres::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT DISTINCT a FROM t",
            expected_starts: &[7],
        },
        #[cfg(feature = "bigquery")]
        DistinctSpanTestCase {
            description: "bigquery",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT DISTINCT a FROM t",
            expected_starts: &[7],
        },
        #[cfg(feature = "databricks")]
        DistinctSpanTestCase {
            description: "databricks",
            parse: |sql| {
                sqlquack_syntax_databricks::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT DISTINCT a FROM t",
            expected_starts: &[7],
        },
        #[cfg(feature = "duckdb")]
        DistinctSpanTestCase {
            description: "duckdb",
            parse: |sql| {
                sqlquack_syntax_duckdb::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT DISTINCT a FROM t",
            expected_starts: &[7],
        },
        #[cfg(feature = "sqlite")]
        DistinctSpanTestCase {
            description: "sqlite",
            parse: |sql| {
                sqlquack_syntax_sqlite::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT DISTINCT a FROM t",
            expected_starts: &[7],
        },
        #[cfg(feature = "clickhouse")]
        DistinctSpanTestCase {
            description: "clickhouse",
            parse: |sql| {
                sqlquack_syntax_clickhouse::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT DISTINCT a FROM t",
            expected_starts: &[7],
        },
        #[cfg(feature = "sqlserver")]
        DistinctSpanTestCase {
            description: "sqlserver",
            parse: |sql| {
                sqlquack_syntax_sqlserver::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT DISTINCT a FROM t",
            expected_starts: &[7],
        },
    ];

    for test_case in test_cases {
        let statements: Vec<Expression> = (test_case.parse)(test_case.sql).unwrap_or_default();
        let mut starts: Vec<usize> = statements
            .iter()
            .flat_map(|statement| statement.dfs())
            .filter_map(Expression::as_select)
            .filter_map(|select| select.distinct_span.0)
            .map(|keyword| keyword.start)
            .collect();
        starts.sort_unstable();
        assert_eq!(
            starts, test_case.expected_starts,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_calls_when_parsed_as_written_then_each_keeps_its_name_and_qualifier() {
    let test_cases = [
        WrittenCallTestCase {
            description: "Snowflake calls in QUALIFY",
            parse: |sql| parse_as_written(sql, Dialect::Snowflake),
            sql: "SELECT id FROM t QUALIFY ROW_NUMBER() OVER (ORDER BY id) = 1",
            expected_calls: &[("ROW_NUMBER", "")],
        },
        WrittenCallTestCase {
            description: "Snowflake nested calls",
            parse: |sql| parse_as_written(sql, Dialect::Snowflake),
            sql: "SELECT TRY_PARSE_JSON(LOWER(x)) FROM t",
            expected_calls: &[("TRY_PARSE_JSON", ""), ("LOWER", "")],
        },
        WrittenCallTestCase {
            description: "a Snowflake qualified call",
            parse: |sql| parse_as_written(sql, Dialect::Snowflake),
            sql: "SELECT db.sch.my_udf(x), util.Parse_Day(d) FROM t",
            expected_calls: &[("my_udf", "db.sch"), ("Parse_Day", "util")],
        },
        WrittenCallTestCase {
            description: "DuckDB calls in QUALIFY",
            parse: |sql| parse_as_written(sql, Dialect::DuckDB),
            sql: "SELECT id FROM t QUALIFY ROW_NUMBER() OVER (ORDER BY id) = 1",
            expected_calls: &[("ROW_NUMBER", "")],
        },
        WrittenCallTestCase {
            description: "DuckDB calls it knows keep their names",
            parse: |sql| parse_as_written(sql, Dialect::DuckDB),
            sql: "SELECT TRY_PARSE_JSON(LOWER(x)), count(*), Sum(x), coalesce(x, 0) FROM t",
            expected_calls: &[
                ("TRY_PARSE_JSON", ""),
                ("LOWER", ""),
                ("count", ""),
                ("Sum", ""),
                ("coalesce", ""),
            ],
        },
        WrittenCallTestCase {
            description: "a DuckDB qualified call",
            parse: |sql| parse_as_written(sql, Dialect::DuckDB),
            sql: "SELECT db.sch.my_udf(x) FROM t",
            expected_calls: &[("my_udf", "db.sch")],
        },
        WrittenCallTestCase {
            description: "a DuckDB method on a value is no qualified call",
            parse: |sql| parse_as_written(sql, Dialect::DuckDB),
            sql: "SELECT (x).upper() FROM t",
            expected_calls: &[],
        },
        WrittenCallTestCase {
            description: "SQLite calls",
            parse: |sql| parse_as_written(sql, Dialect::SQLite),
            sql: "SELECT LOWER(x), Count(*) FROM t",
            expected_calls: &[("LOWER", ""), ("Count", "")],
        },
        #[cfg(feature = "postgres")]
        WrittenCallTestCase {
            description: "postgres",
            parse: |sql| {
                sqlquack_syntax_postgres::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT LOWER(x), Count(*), util.Parse_Day(d) FROM t",
            expected_calls: &[("LOWER", ""), ("Count", ""), ("Parse_Day", "util")],
        },
        #[cfg(feature = "bigquery")]
        WrittenCallTestCase {
            description: "bigquery",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT LOWER(x), Count(*), util.Parse_Day(d) FROM t",
            expected_calls: &[("LOWER", ""), ("Count", ""), ("Parse_Day", "util")],
        },
        #[cfg(feature = "databricks")]
        WrittenCallTestCase {
            description: "databricks",
            parse: |sql| {
                sqlquack_syntax_databricks::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT LOWER(x), Count(*), util.Parse_Day(d) FROM t",
            expected_calls: &[("LOWER", ""), ("Count", ""), ("Parse_Day", "util")],
        },
        WrittenCallTestCase {
            description: "a table function in FROM",
            parse: |sql| parse_as_written(sql, Dialect::DuckDB),
            sql: "SELECT * FROM Generate_Series(1, 3) AS g(n)",
            expected_calls: &[("Generate_Series", "")],
        },
        WrittenCallTestCase {
            description: "a qualified table function in FROM",
            parse: |sql| parse_as_written(sql, Dialect::DuckDB),
            sql: "SELECT * FROM util.Split_Day(1)",
            expected_calls: &[("Split_Day", "util")],
        },
        WrittenCallTestCase {
            description: "a lateral table function after a comma",
            parse: |sql| parse_as_written(sql, Dialect::Snowflake),
            sql: "SELECT * FROM t, LATERAL Flatten(input => t.items) AS f",
            expected_calls: &[("Flatten", "")],
        },
        WrittenCallTestCase {
            description: "a call without parentheses",
            parse: |sql| parse_as_written(sql, Dialect::Snowflake),
            sql: "SELECT current_date FROM t",
            expected_calls: &[("current_date", "")],
        },
    ];

    for test_case in test_cases {
        let statements: Vec<Expression> = (test_case.parse)(test_case.sql).unwrap_or_default();
        let mut calls: Vec<(usize, String, String)> = statements
            .iter()
            .flat_map(|statement| statement.dfs())
            .filter_map(|node| node.written_call(test_case.sql))
            .map(|call| (call.span.start, call.name, call.qualifier.join(".")))
            .collect();
        calls.sort_unstable();
        let written: Vec<(&str, &str)> = calls
            .iter()
            .map(|(_, name, qualifier)| (name.as_str(), qualifier.as_str()))
            .collect();
        assert_eq!(
            written, test_case.expected_calls,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_row_limits_when_parsed_located_then_each_query_records_its_keyword() {
    let test_cases = [
        RowLimitSpanTestCase {
            description: "a select",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT a FROM t ORDER BY a LIMIT 5",
            expected_starts: &[27],
        },
        RowLimitSpanTestCase {
            description: "an inner and an outer limit",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT a FROM (SELECT a FROM t LIMIT 9) AS s LIMIT 5 OFFSET 2",
            expected_starts: &[31, 45],
        },
        RowLimitSpanTestCase {
            description: "FETCH alone",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT a FROM t FETCH FIRST 5 ROWS ONLY",
            expected_starts: &[16],
        },
        RowLimitSpanTestCase {
            description: "OFFSET alone",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT a FROM t OFFSET 5",
            expected_starts: &[16],
        },
        RowLimitSpanTestCase {
            description: "a set operation takes the limit after its last branch",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT a FROM t UNION ALL SELECT a FROM u LIMIT 5",
            expected_starts: &[42],
        },
        RowLimitSpanTestCase {
            description: "a limit after a parenthesised last branch",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT a FROM t UNION ALL (SELECT a FROM u) LIMIT 5",
            expected_starts: &[44],
        },
        RowLimitSpanTestCase {
            description: "a limit after a parenthesised query",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "(SELECT a FROM t) LIMIT 5",
            expected_starts: &[18],
        },
        RowLimitSpanTestCase {
            description: "INTERSECT passes the limit to the looser operation",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1 UNION SELECT 2 INTERSECT SELECT 3 LIMIT 5",
            expected_starts: &[43],
        },
        RowLimitSpanTestCase {
            description: "a column named like the keyword is no limit",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT t.offset FROM t",
            expected_starts: &[],
        },
        #[cfg(feature = "postgres")]
        RowLimitSpanTestCase {
            description: "postgres select",
            parse: |sql| {
                sqlquack_syntax_postgres::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT a FROM t ORDER BY a LIMIT 5",
            expected_starts: &[27],
        },
        #[cfg(feature = "postgres")]
        RowLimitSpanTestCase {
            description: "postgres set operation",
            parse: |sql| {
                sqlquack_syntax_postgres::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT a FROM t UNION ALL SELECT a FROM u LIMIT 5",
            expected_starts: &[42],
        },
        #[cfg(feature = "bigquery")]
        RowLimitSpanTestCase {
            description: "bigquery select",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT a FROM t ORDER BY a LIMIT 5",
            expected_starts: &[27],
        },
        #[cfg(feature = "bigquery")]
        RowLimitSpanTestCase {
            description: "bigquery set operation",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT a FROM t UNION ALL SELECT a FROM u LIMIT 5",
            expected_starts: &[42],
        },
        #[cfg(feature = "databricks")]
        RowLimitSpanTestCase {
            description: "databricks select",
            parse: |sql| {
                sqlquack_syntax_databricks::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT a FROM t ORDER BY a LIMIT 5",
            expected_starts: &[27],
        },
        #[cfg(feature = "databricks")]
        RowLimitSpanTestCase {
            description: "databricks set operation",
            parse: |sql| {
                sqlquack_syntax_databricks::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT a FROM t UNION ALL SELECT a FROM u LIMIT 5",
            expected_starts: &[42],
        },
        #[cfg(feature = "duckdb")]
        RowLimitSpanTestCase {
            description: "duckdb select",
            parse: |sql| {
                sqlquack_syntax_duckdb::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT a FROM t ORDER BY a LIMIT 5",
            expected_starts: &[27],
        },
        #[cfg(feature = "duckdb")]
        RowLimitSpanTestCase {
            description: "duckdb set operation",
            parse: |sql| {
                sqlquack_syntax_duckdb::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT a FROM t UNION ALL SELECT a FROM u LIMIT 5",
            expected_starts: &[42],
        },
        #[cfg(feature = "sqlite")]
        RowLimitSpanTestCase {
            description: "sqlite select",
            parse: |sql| {
                sqlquack_syntax_sqlite::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT a FROM t ORDER BY a LIMIT 5",
            expected_starts: &[27],
        },
        #[cfg(feature = "sqlite")]
        RowLimitSpanTestCase {
            description: "sqlite set operation",
            parse: |sql| {
                sqlquack_syntax_sqlite::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT a FROM t UNION ALL SELECT a FROM u LIMIT 5",
            expected_starts: &[42],
        },
        #[cfg(feature = "clickhouse")]
        RowLimitSpanTestCase {
            description: "clickhouse",
            parse: |sql| {
                sqlquack_syntax_clickhouse::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT a FROM t ORDER BY a LIMIT 5",
            expected_starts: &[27],
        },
        #[cfg(feature = "databricks")]
        RowLimitSpanTestCase {
            description: "databricks parenthesised query",
            parse: |sql| {
                sqlquack_syntax_databricks::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "(SELECT a FROM t) LIMIT 5",
            expected_starts: &[18],
        },
        #[cfg(feature = "sqlserver")]
        RowLimitSpanTestCase {
            description: "sqlserver",
            parse: |sql| {
                sqlquack_syntax_sqlserver::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT a FROM t ORDER BY a OFFSET 5 ROWS",
            expected_starts: &[27],
        },
    ];

    for test_case in test_cases {
        let statements: Vec<Expression> = (test_case.parse)(test_case.sql).unwrap_or_default();
        let mut starts: Vec<usize> = statements
            .iter()
            .flat_map(|statement| statement.dfs())
            .filter_map(|node| node.row_limit_span())
            .filter_map(|keyword| keyword.0)
            .map(|keyword| keyword.start)
            .collect();
        starts.sort_unstable();
        assert_eq!(
            starts, test_case.expected_starts,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_case_expressions_when_parsed_located_then_each_records_its_keywords() {
    let test_cases = [
        CaseKeywordsTestCase {
            description: "Snowflake",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT CASE a WHEN 1 THEN 'x' WHEN 2 THEN 'y' ELSE CASE WHEN b = 1 THEN 'z' END END FROM t",
            expected_keywords: &[
                "CASE WHEN THEN WHEN THEN ELSE END ELSE CASE WHEN b = 1 THEN 'z' END",
                "CASE WHEN THEN END",
            ],
        },
        CaseKeywordsTestCase {
            description: "DuckDB through the shared parser",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT CASE a WHEN 1 THEN 'x' WHEN 2 THEN 'y' ELSE CASE WHEN b = 1 THEN 'z' END END FROM t",
            expected_keywords: &[
                "CASE WHEN THEN WHEN THEN ELSE END ELSE CASE WHEN b = 1 THEN 'z' END",
                "CASE WHEN THEN END",
            ],
        },
        CaseKeywordsTestCase {
            description: "SQLite through the shared parser",
            parse: |sql| parse_source(sql, Dialect::SQLite).ok(),
            sql: "SELECT CASE a WHEN 1 THEN 'x' WHEN 2 THEN 'y' ELSE CASE WHEN b = 1 THEN 'z' END END FROM t",
            expected_keywords: &[
                "CASE WHEN THEN WHEN THEN ELSE END ELSE CASE WHEN b = 1 THEN 'z' END",
                "CASE WHEN THEN END",
            ],
        },
        CaseKeywordsTestCase {
            description: "after a character of three bytes",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT '注', case when a then 1 end FROM t",
            expected_keywords: &["case when then end"],
        },
        #[cfg(feature = "postgres")]
        CaseKeywordsTestCase {
            description: "postgres",
            parse: |sql| {
                sqlquack_syntax_postgres::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT CASE a WHEN 1 THEN 'x' WHEN 2 THEN 'y' ELSE CASE WHEN b = 1 THEN 'z' END END FROM t",
            expected_keywords: &[
                "CASE WHEN THEN WHEN THEN ELSE END ELSE CASE WHEN b = 1 THEN 'z' END",
                "CASE WHEN THEN END",
            ],
        },
        #[cfg(feature = "bigquery")]
        CaseKeywordsTestCase {
            description: "bigquery",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT CASE a WHEN 1 THEN 'x' WHEN 2 THEN 'y' ELSE CASE WHEN b = 1 THEN 'z' END END FROM t",
            expected_keywords: &[
                "CASE WHEN THEN WHEN THEN ELSE END ELSE CASE WHEN b = 1 THEN 'z' END",
                "CASE WHEN THEN END",
            ],
        },
        #[cfg(feature = "databricks")]
        CaseKeywordsTestCase {
            description: "databricks",
            parse: |sql| {
                sqlquack_syntax_databricks::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT CASE a WHEN 1 THEN 'x' WHEN 2 THEN 'y' ELSE CASE WHEN b = 1 THEN 'z' END END FROM t",
            expected_keywords: &[
                "CASE WHEN THEN WHEN THEN ELSE END ELSE CASE WHEN b = 1 THEN 'z' END",
                "CASE WHEN THEN END",
            ],
        },
        #[cfg(feature = "duckdb")]
        CaseKeywordsTestCase {
            description: "duckdb",
            parse: |sql| {
                sqlquack_syntax_duckdb::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT CASE a WHEN 1 THEN 'x' WHEN 2 THEN 'y' ELSE CASE WHEN b = 1 THEN 'z' END END FROM t",
            expected_keywords: &[
                "CASE WHEN THEN WHEN THEN ELSE END ELSE CASE WHEN b = 1 THEN 'z' END",
                "CASE WHEN THEN END",
            ],
        },
        #[cfg(feature = "sqlite")]
        CaseKeywordsTestCase {
            description: "sqlite",
            parse: |sql| {
                sqlquack_syntax_sqlite::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT CASE a WHEN 1 THEN 'x' WHEN 2 THEN 'y' ELSE CASE WHEN b = 1 THEN 'z' END END FROM t",
            expected_keywords: &[
                "CASE WHEN THEN WHEN THEN ELSE END ELSE CASE WHEN b = 1 THEN 'z' END",
                "CASE WHEN THEN END",
            ],
        },
        #[cfg(feature = "clickhouse")]
        CaseKeywordsTestCase {
            description: "clickhouse",
            parse: |sql| {
                sqlquack_syntax_clickhouse::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT CASE a WHEN 1 THEN 'x' WHEN 2 THEN 'y' ELSE CASE WHEN b = 1 THEN 'z' END END FROM t",
            expected_keywords: &[
                "CASE WHEN THEN WHEN THEN ELSE END ELSE CASE WHEN b = 1 THEN 'z' END",
                "CASE WHEN THEN END",
            ],
        },
        #[cfg(feature = "sqlserver")]
        CaseKeywordsTestCase {
            description: "sqlserver",
            parse: |sql| {
                sqlquack_syntax_sqlserver::parse(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT CASE a WHEN 1 THEN 'x' WHEN 2 THEN 'y' ELSE CASE WHEN b = 1 THEN 'z' END END FROM t",
            expected_keywords: &[
                "CASE WHEN THEN WHEN THEN ELSE END ELSE CASE WHEN b = 1 THEN 'z' END",
                "CASE WHEN THEN END",
            ],
        },
    ];

    for test_case in test_cases {
        let statements: Vec<Expression> = (test_case.parse)(test_case.sql).unwrap_or_default();
        assert_eq!(
            case_keywords(&statements, test_case.sql),
            test_case.expected_keywords,
            "{}",
            test_case.description
        );
    }
}
