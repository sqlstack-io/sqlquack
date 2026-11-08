//! SQLite's scope rules, and the types and names it records, as sqlite3 3.53 prepares queries.

use sqlquack_syntax::Dialect;

use crate::dialects::constants::SQLITE;
use crate::lineage::_helpers::findings::statement::statement_findings;
use crate::lineage::models::{LineageContext, Nullability};
use crate::lineage::tests::helpers::{
    deep_sqlite_analysis, found_findings, generic_calls, nested_derived_tables, owned_findings,
    sqlite_outputs, sqlite_shop_schema,
};
use crate::lineage::tests::test_types::{
    DeepSqliteTestCase, QueryFindingsTestCase, SqliteOutputsTestCase,
};

#[test]
fn given_a_sqlite_query_sqlite_rejects_when_checking_then_the_broken_rule_is_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "two CTEs under one name",
            sql: "WITH x AS (SELECT 1 AS a), x AS (SELECT 2 AS a) SELECT a FROM x",
            expected_findings: &[("E233", "Duplicate CTE name 'x'", 55)],
        },
        QueryFindingsTestCase {
            description: "fewer CTE column aliases than its outputs",
            sql: "WITH x(a) AS (SELECT 1, 2) SELECT a FROM x",
            expected_findings: &[(
                "E223",
                "CTE column alias count does not match its output column count",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "a ranking function without OVER",
            sql: "SELECT ROW_NUMBER() AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 0)],
        },
        QueryFindingsTestCase {
            description: "a column as the LIMIT",
            sql: "SELECT id FROM orders LIMIT quantity",
            expected_findings: &[(
                "E234",
                "LIMIT/OFFSET must be a non-negative constant numeric expression",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "UNION branches of different widths",
            sql: "SELECT id, status FROM orders UNION ALL SELECT id FROM orders",
            expected_findings: &[(
                "E216",
                "UNION operands return different column counts: left 2, right 1",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "an unknown column",
            sql: "SELECT nope FROM orders",
            expected_findings: &[("E201", "Unknown column 'nope' in table 'orders'", 7)],
        },
    ];
    let schema = sqlite_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::SQLite, &SQLITE, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_sqlite_query_sqlite_accepts_when_checking_then_nothing_is_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "a bare column in an aggregate query, read from any row of its group",
            sql: "SELECT id, status, SUM(price) AS total FROM orders GROUP BY id",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "HAVING reads a column neither grouped nor aggregated",
            sql: "SELECT status FROM orders GROUP BY id HAVING price > 1",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "two FROM items under one alias, no column read through it",
            sql: "SELECT 1 AS one FROM orders AS a, customers AS a",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a negative LIMIT, which SQLite reads as no limit",
            sql: "SELECT id FROM orders LIMIT -1",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a text column as a predicate and text plus a number",
            sql: "SELECT status + 1 AS bumped FROM orders WHERE status",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a quoted name in another case, which SQLite matches",
            sql: "SELECT \"STATUS\", COUNT(*) AS n FROM orders GROUP BY status",
            expected_findings: &[],
        },
    ];
    let schema = sqlite_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::SQLite, &SQLITE, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sqlite_outputs_when_typing_then_only_one_storage_class_values_are_typed() {
    let test_cases = [
        SqliteOutputsTestCase {
            description: "literals have the storage class typeof() reports",
            sql: "SELECT 1 AS i, 1.5 AS r, 1e3 AS e, 'a' AS t, NULL AS n",
            expected_outputs: &[
                ("i", Some("INTEGER"), Nullability::NonNull),
                ("r", Some("REAL"), Nullability::NonNull),
                ("e", Some("REAL"), Nullability::NonNull),
                ("t", Some("TEXT"), Nullability::NonNull),
                ("n", None, Nullability::Nullable),
            ],
        },
        SqliteOutputsTestCase {
            description: "casts type by affinity; NUMERIC affinity is INTEGER or REAL by value",
            sql: "SELECT CAST(price AS VARCHAR(10)) AS t, CAST(status AS BIGINT) AS i, \
                  CAST(id AS DOUBLE) AS r, CAST(price AS DECIMAL(10,2)) AS d, \
                  CAST(status AS DATE) AS dt FROM orders",
            expected_outputs: &[
                ("t", Some("TEXT"), Nullability::Unknown),
                ("i", Some("INTEGER"), Nullability::Unknown),
                ("r", Some("REAL"), Nullability::Unknown),
                ("d", None, Nullability::Unknown),
                ("dt", None, Nullability::Unknown),
            ],
        },
        SqliteOutputsTestCase {
            description: "computed values have no one type, and a division can be NULL",
            sql: "SELECT SUM(quantity) AS s, quantity / 0 AS q, UPPER(status) AS u FROM orders",
            expected_outputs: &[
                ("s", None, Nullability::Unknown),
                ("q", None, Nullability::Unknown),
                ("u", None, Nullability::Unknown),
            ],
        },
        SqliteOutputsTestCase {
            description: "a column keeps its declared type; an unaliased expression is unnamed",
            sql: "SELECT price, quantity * 2 FROM orders",
            expected_outputs: &[
                ("price", Some("REAL"), Nullability::Unknown),
                ("", None, Nullability::Unknown),
            ],
        },
        SqliteOutputsTestCase {
            description: "VALUES columns are named column1, column2",
            sql: "SELECT * FROM (VALUES (1, 'a'))",
            expected_outputs: &[
                ("column1", Some("INTEGER"), Nullability::NonNull),
                ("column2", Some("TEXT"), Nullability::NonNull),
            ],
        },
    ];
    let schema = sqlite_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            sqlite_outputs(&schema, test_case.sql),
            test_case
                .expected_outputs
                .iter()
                .map(|(name, data_type, nullability)| {
                    (
                        (*name).to_owned(),
                        data_type.map(str::to_owned),
                        *nullability,
                    )
                })
                .collect::<Vec<(String, Option<String>, Nullability)>>(),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sqlite_window_only_calls_by_name_when_checking_then_each_without_over_is_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "row_number without OVER, as sqlite3 refuses it",
            sql: "SELECT row_number() AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "rank without OVER, as sqlite3 refuses it",
            sql: "SELECT rank() AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "dense_rank without OVER, as sqlite3 refuses it",
            sql: "SELECT dense_rank() AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "percent_rank without OVER, as sqlite3 refuses it",
            sql: "SELECT percent_rank() AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "cume_dist without OVER, as sqlite3 refuses it",
            sql: "SELECT cume_dist() AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "ntile without OVER, as sqlite3 refuses it",
            sql: "SELECT ntile(2) AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "lag without OVER, as sqlite3 refuses it",
            sql: "SELECT lag(id) AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "lead without OVER, as sqlite3 refuses it",
            sql: "SELECT lead(id) AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "first_value without OVER, as sqlite3 refuses it",
            sql: "SELECT first_value(id) AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "last_value without OVER, as sqlite3 refuses it",
            sql: "SELECT last_value(id) AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "nth_value without OVER, as sqlite3 refuses it",
            sql: "SELECT nth_value(id, 1) AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "an ordinary call by name",
            sql: "SELECT upper(status) AS r FROM orders",
            expected_findings: &[],
        },
    ];
    let schema = sqlite_shop_schema();
    for test_case in test_cases {
        let context = LineageContext {
            profile: &SQLITE,
            schema: &schema,
            sql: Some(test_case.sql),
        };
        let statement = generic_calls(test_case.sql);
        assert_eq!(
            statement_findings(&statement, context)
                .into_iter()
                .map(|finding| (
                    finding.code,
                    finding.message,
                    finding.span.map_or(0, |span| span.start)
                ))
                .collect::<Vec<_>>(),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_sqlite_tree_nested_to_the_parser_limit_when_analysed_on_a_small_stack_then_it_completes()
{
    let test_cases = [
        DeepSqliteTestCase {
            description: "998 nested derived tables, a tree about 2,000 levels deep",
            sql: nested_derived_tables(998),
            expected_result: Ok((1998, 1, 0)),
        },
        DeepSqliteTestCase {
            description: "990 chained additions in one output",
            sql: format!("SELECT 1{} AS a FROM orders", " + 1".repeat(990)),
            expected_result: Ok((993, 1, 0)),
        },
        DeepSqliteTestCase {
            description: "derived tables nested past the parser limit are refused",
            sql: nested_derived_tables(1100),
            expected_result: Err("the statement nests too deeply"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            deep_sqlite_analysis(test_case.sql),
            test_case.expected_result.map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
