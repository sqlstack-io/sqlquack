//! TOP and FETCH: the first rows they keep, and the forms LIMIT cannot express.

use crate::helpers::{convert_outcome, converted_first_value, run_convert};
use crate::test_types::{ConvertTestCase, ValueTestCase};

#[test]
fn given_top_or_fetch_that_limit_cannot_express_when_converting_then_it_is_refused() {
    let test_cases = [
        ConvertTestCase {
            description: "TOP with PERCENT refuses: LIMIT keeps a count, not a share",
            sql: "WITH t AS (SELECT 1 AS order_id) SELECT TOP 10 PERCENT order_id FROM t",
            flags: &[],
            mention: "TOP with PERCENT or WITH TIES, which LIMIT does not express",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "FETCH WITH TIES refuses: LIMIT drops the tied rows",
            sql: "WITH t AS (SELECT 1 AS order_id) \
                  SELECT order_id FROM t ORDER BY order_id FETCH FIRST 2 ROWS WITH TIES",
            flags: &[],
            mention: "FETCH with PERCENT or WITH TIES, which LIMIT does not express",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "TOP together with LIMIT refuses: no recording shows both",
            sql: "WITH t AS (SELECT 1 AS order_id) SELECT TOP 2 order_id FROM t LIMIT 3",
            flags: &[],
            mention: "more than one of LIMIT, TOP and FETCH",
            expected_outcome: convert_outcome(1, false),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            run_convert(test_case.sql, test_case.flags, test_case.mention),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_top_or_fetch_when_converted_and_run_on_duckdb_then_the_first_rows_are_kept() {
    let test_cases = [
        ValueTestCase {
            description: "TOP keeps the first rows after ORDER BY",
            sql: "WITH t AS (SELECT column1 AS a FROM VALUES ('a'), ('a'), ('b'), ('c')) \
                  SELECT MAX(a) AS m FROM (SELECT TOP 3 a FROM t ORDER BY a)",
            expected_value: "b",
        },
        ValueTestCase {
            description: "DISTINCT TOP counts distinct rows",
            sql: "WITH t AS (SELECT column1 AS a FROM VALUES ('a'), ('a'), ('b'), ('c')) \
                  SELECT MAX(a) AS m FROM (SELECT DISTINCT TOP 2 a FROM t ORDER BY a)",
            expected_value: "b",
        },
        ValueTestCase {
            description: "OFFSET ... FETCH NEXT ... ROWS ONLY skips, then keeps",
            sql: "WITH t AS (SELECT column1 AS a FROM VALUES ('a'), ('a'), ('b'), ('c')) \
                  SELECT MIN(a) AS m FROM (SELECT a FROM t ORDER BY a OFFSET 2 ROWS FETCH NEXT 2 ROWS ONLY)",
            expected_value: "b",
        },
        ValueTestCase {
            description: "FETCH FIRST 1 ROW without ONLY keeps one row",
            sql: "WITH t AS (SELECT column1 AS a FROM VALUES ('a'), ('a'), ('b'), ('c')) \
                  SELECT MAX(a) AS m FROM (SELECT a FROM t ORDER BY a FETCH FIRST 1 ROW)",
            expected_value: "a",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            converted_first_value(test_case.sql),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}
