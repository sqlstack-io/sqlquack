use sqlquack_syntax::Dialect;

use crate::dialects::constants::{DUCKDB, POSTGRES, SNOWFLAKE};
use crate::lineage::tests::helpers::{
    found_findings, owned_findings, typed_duckdb_shop_schema, typed_shop_schema,
};
use crate::lineage::tests::test_types::QueryFindingsTestCase;

#[test]
fn given_a_duckdb_select_when_checking_its_scope_rules_then_broken_rules_are_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "HAVING reads a column neither grouped nor aggregated",
            sql: "SELECT status, COUNT(*) AS n FROM orders GROUP BY status HAVING quantity > 0",
            expected_findings: &[(
                "E230",
                "Column must be grouped or used in an aggregate function",
                64,
            )],
        },
        QueryFindingsTestCase {
            description: "an aggregate inside another aggregate",
            sql: "SELECT SUM(SUM(quantity)) AS s FROM orders",
            expected_findings: &[(
                "E231",
                "Aggregate function is not allowed in this clause or inside another aggregate",
                15,
            )],
        },
        QueryFindingsTestCase {
            description: "a ranking function without OVER",
            sql: "SELECT ROW_NUMBER() AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 0)],
        },
        QueryFindingsTestCase {
            description: "CUME_DIST without OVER",
            sql: "SELECT CUME_DIST() AS w FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 0)],
        },
        QueryFindingsTestCase {
            description: "PERCENT_RANK without OVER",
            sql: "SELECT PERCENT_RANK() AS w FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 0)],
        },
        QueryFindingsTestCase {
            description: "NTH_VALUE without OVER",
            sql: "SELECT NTH_VALUE(id, 1) AS w FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 17)],
        },
        QueryFindingsTestCase {
            description: "CUME_DIST, PERCENT_RANK and NTH_VALUE under OVER",
            sql: "SELECT CUME_DIST() OVER (ORDER BY id) AS c, PERCENT_RANK() OVER (ORDER BY id) AS p, \
                  NTH_VALUE(id, 1) OVER (ORDER BY id) AS n, \
                  NTH_VALUE(id, 2) FROM LAST IGNORE NULLS OVER (ORDER BY id) AS l, \
                  NTH_VALUE(id, 1) FROM FIRST RESPECT NULLS OVER (ORDER BY id) AS f, \
                  NTH_VALUE(id, 1) IGNORE NULLS OVER (ORDER BY id) AS i FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "two FROM items under one alias",
            sql: "SELECT o.id FROM orders o JOIN customers o ON TRUE",
            expected_findings: &[("E233", "Duplicate table alias 'o'", 0)],
        },
        QueryFindingsTestCase {
            description: "a negative LIMIT",
            sql: "SELECT id FROM orders LIMIT -1",
            expected_findings: &[(
                "E234",
                "LIMIT/OFFSET must be a non-negative constant numeric expression",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "more derived-table column aliases than columns",
            sql: "SELECT a FROM (SELECT id, quantity FROM orders) AS r(a, b, c)",
            expected_findings: &[(
                "E223",
                "Derived-table column alias count exceeds its output column count",
                59,
            )],
        },
        QueryFindingsTestCase {
            description: "a grouped query whose reads are all grouped or aggregated",
            sql: "SELECT status, COUNT(*) AS n FROM orders GROUP BY 1 HAVING COUNT(*) > 1 ORDER BY n",
            expected_findings: &[],
        },
    ];
    let schema = typed_duckdb_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::DuckDB, &DUCKDB, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_duckdb_query_when_checking_its_types_then_definite_type_errors_are_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "an integer plus a timestamp",
            sql: "SELECT quantity + ordered_at AS total FROM orders",
            expected_findings: &[(
                "E212",
                "Arithmetic operation expects numeric-compatible operands, found integer and timestamp",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "an integer compared with a timestamp",
            sql: "SELECT quantity > ordered_at AS compared FROM orders",
            expected_findings: &[(
                "E217",
                "Incompatible comparison between integer and timestamp",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a text literal that is not a number, left for SQLBuild to place",
            sql: "SELECT 1 AS payment_id WHERE 'open' > 1",
            expected_findings: &[(
                "W213",
                "Comparison contains an invalid coercible literal",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "UPPER of an integer",
            sql: "SELECT UPPER(quantity) AS u FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'upper' argument 1 expects String, found integer",
                13,
            )],
        },
        QueryFindingsTestCase {
            description: "set-operation branches of different families",
            sql: "SELECT id FROM orders UNION ALL SELECT ordered_at FROM orders",
            expected_findings: &[(
                "W214",
                "UNION column 1 has incompatible types: integer vs timestamp",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "a timestamp cast to an integer",
            sql: "SELECT CAST(ordered_at AS INTEGER) AS c FROM orders",
            expected_findings: &[("W213", "Cannot cast timestamp to integer", 12)],
        },
        QueryFindingsTestCase {
            description: "CASE results that do not unify",
            sql: "SELECT CASE WHEN quantity > 0 THEN status ELSE ordered_at END AS c FROM orders",
            expected_findings: &[("E213", "Cannot unify string and timestamp", 17)],
        },
        QueryFindingsTestCase {
            description: "a keyword-only function called with parentheses, left unplaced",
            sql: "SELECT CURRENT_TIMESTAMP() AS created_at",
            expected_findings: &[(
                "E202",
                "Unknown function 'CURRENT_TIMESTAMP' for dialect DuckDB",
                0,
            )],
        },
    ];
    let schema = typed_duckdb_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::DuckDB, &DUCKDB, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_snowflake_query_when_checking_built_in_calls_and_types_then_definite_errors_are_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "NVL with one argument",
            sql: "SELECT NVL(id) AS d FROM orders",
            expected_findings: &[(
                "E203",
                "Invalid arity for built-in function 'nvl': got 1",
                11,
            )],
        },
        QueryFindingsTestCase {
            description: "ROW_NUMBER with an argument",
            sql: "SELECT ROW_NUMBER(id) OVER (ORDER BY id) AS d FROM orders",
            expected_findings: &[(
                "E203",
                "Invalid arity for built-in function 'row_number': got 1",
                18,
            )],
        },
        QueryFindingsTestCase {
            description: "CUME_DIST without OVER, as Snowflake parses it",
            sql: "SELECT CUME_DIST() AS w FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "PERCENT_RANK without OVER, as Snowflake parses it",
            sql: "SELECT PERCENT_RANK() AS w FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "NTH_VALUE without OVER, as Snowflake parses it",
            sql: "SELECT NTH_VALUE(id, 1) AS w FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "CUME_DIST, PERCENT_RANK and NTH_VALUE under OVER",
            sql: "SELECT CUME_DIST() OVER (ORDER BY id) AS c, PERCENT_RANK() OVER (ORDER BY id) AS p, \
                  NTH_VALUE(id, 1) OVER (ORDER BY id) AS n, \
                  NTH_VALUE(id, 2) FROM LAST IGNORE NULLS OVER (ORDER BY id) AS l, \
                  NTH_VALUE(id, 1) FROM FIRST RESPECT NULLS OVER (ORDER BY id) AS f, \
                  NTH_VALUE(id, 1) IGNORE NULLS OVER (ORDER BY id) AS i FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a number condition in WHERE",
            sql: "SELECT id FROM orders WHERE amount",
            expected_findings: &[(
                "E211",
                "WHERE expects a boolean predicate, found numeric",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "NUMBER then VARCHAR branches convert at run time",
            sql: "SELECT id FROM orders UNION ALL SELECT status FROM orders",
            expected_findings: &[(
                "W214",
                "Set-operation column 1 may fail during runtime conversion: accumulated type NUMBER, next type VARCHAR",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "DISTINCT ordered by a column it does not select",
            sql: "SELECT DISTINCT id FROM orders ORDER BY status",
            expected_findings: &[(
                "E201",
                "DISTINCT ORDER BY references a non-selected column",
                40,
            )],
        },
        QueryFindingsTestCase {
            description: "more CTE column aliases than columns",
            sql: "WITH c (a, b) AS (SELECT id FROM orders) SELECT a FROM c",
            expected_findings: &[(
                "E223",
                "CTE column alias count does not match its output column count",
                25,
            )],
        },
        QueryFindingsTestCase {
            description: "BY NAME branches pair columns by name, not position",
            sql: "SELECT id, status FROM orders UNION ALL BY NAME SELECT status, id FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a number plus text, which Snowflake converts",
            sql: "SELECT id + status AS d FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "aggregates the parse keeps as plain calls read ungrouped columns",
            sql: "SELECT status, ARRAY_UNIQUE_AGG(id) AS a, BITOR_AGG(id) AS b, \
                  BITXOR_AGG(id) AS c, MINHASH(5, id) AS m FROM orders GROUP BY status",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a schema-qualified function call is not a column read",
            sql: "SELECT util.shop_score(id) AS s, db.util.shop_score(id) AS t FROM orders \
                  WHERE util.is_ok(status)",
            expected_findings: &[],
        },
    ];
    let schema = typed_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::Snowflake, &SNOWFLAKE, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_call_or_type_the_checks_cannot_prove_when_checking_then_nothing_is_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "unknown and declared functions are never errors",
            sql: "SELECT TO_DATE('2026-01-01') AS d, shop_score(status) AS s, mystery(1, 2) AS m FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a declared function's arguments are not checked",
            sql: "SELECT shop_score(ordered_at) + 1 AS s FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "columns of an unknown relation have no type to check",
            sql: "SELECT x.anything + 1 AS u, x.other > ordered_at AS c FROM orders, events x",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an unknown function's result has no type to check",
            sql: "SELECT mystery(quantity) + ordered_at AS u FROM orders",
            expected_findings: &[],
        },
    ];
    let schema = typed_duckdb_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::DuckDB, &DUCKDB, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_duckdb_query_when_checking_subqueries_keys_frames_and_the_catalog_then_mismatches_are_found()
 {
    let test_cases = [
        QueryFindingsTestCase {
            description: "an IN subquery of two columns",
            sql: "SELECT id FROM orders WHERE id IN (SELECT id, name FROM customers)",
            expected_findings: &[("E216", "IN subquery returns 2 columns, expected 1", 28)],
        },
        QueryFindingsTestCase {
            description: "an IN subquery of text for a timestamp",
            sql: "SELECT id FROM orders WHERE ordered_at IN (SELECT name FROM customers)",
            expected_findings: &[("E217", "IN subquery has incompatible output type", 28)],
        },
        QueryFindingsTestCase {
            description: "a USING key of a number and text, which converts at run time",
            sql: "SELECT * FROM (SELECT id FROM orders) a JOIN (SELECT name AS id FROM customers) b USING (id)",
            expected_findings: &[("W210", "JOIN USING column 'id' has incompatible types", 0)],
        },
        QueryFindingsTestCase {
            description: "a recursive branch of text for a number anchor",
            sql: "WITH RECURSIVE r(n) AS (SELECT 1 UNION ALL SELECT 'x' FROM r WHERE n < 3) SELECT n FROM r",
            expected_findings: &[(
                "W214",
                "Recursive branch cannot convert to the anchor column type",
                81,
            )],
        },
        QueryFindingsTestCase {
            description: "a RANGE offset over a text order",
            sql: "SELECT SUM(quantity) OVER (ORDER BY status RANGE BETWEEN 1 PRECEDING AND CURRENT ROW) AS s FROM orders",
            expected_findings: &[(
                "E232",
                "RANGE offset requires a numeric or temporal ordering expression",
                11,
            )],
        },
        QueryFindingsTestCase {
            description: "a row value against a one-column subquery",
            sql: "SELECT id FROM orders WHERE (id, quantity) = (SELECT id FROM customers)",
            expected_findings: &[("E216", "Row comparison has different column counts", 29)],
        },
        QueryFindingsTestCase {
            description: "a scalar subquery's text compared with a timestamp",
            sql: "SELECT id FROM orders WHERE ordered_at > (SELECT MAX(name) FROM customers)",
            expected_findings: &[(
                "E217",
                "Incompatible comparison between timestamp and string",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "argument counts the catalog does not accept",
            sql: "SELECT SUBSTRING(status) AS x, COALESCE() AS y FROM orders",
            expected_findings: &[
                (
                    "E203",
                    "Invalid arity for function 'SUBSTRING': got 1, expected 2..3",
                    17,
                ),
                (
                    "E203",
                    "Invalid arity for function 'coalesce': got 0, expected 1+",
                    0,
                ),
                ("E203", "COALESCE requires at least one argument", 0),
            ],
        },
        QueryFindingsTestCase {
            description: "a call the catalog does not know is not checked",
            sql: "SELECT order_band(status, 1, 2, 3) AS b FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an alias names no unaliased derived table",
            sql: "SELECT x.id FROM (SELECT id FROM orders)",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'x' referenced by column 'id'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "two items under one name count once for ambiguity",
            sql: "SELECT id FROM orders JOIN orders ON id = id",
            expected_findings: &[("E233", "Duplicate table alias 'orders'", 0)],
        },
        QueryFindingsTestCase {
            description: "an alias of a call is checked under its canonical name",
            sql: "SELECT DATE_DIFF('day', status, ordered_at) AS x FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'datediff' argument 2 expects Temporal, found string",
                24,
            )],
        },
        QueryFindingsTestCase {
            description: "EXTRACT's value is its second argument",
            sql: "SELECT EXTRACT(year FROM status) AS x FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'extract' argument 2 expects a temporal argument, found string",
                25,
            )],
        },
    ];
    let schema = typed_duckdb_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::DuckDB, &DUCKDB, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_postgres_query_when_checking_scope_rules_then_only_postgres_errors_are_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "DISTINCT ON may order by columns it does not select",
            sql: "SELECT DISTINCT ON (status) status, quantity FROM orders \
                  ORDER BY status, ordered_at DESC",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a CTE column list may name fewer columns than the CTE outputs",
            sql: "WITH c(a) AS (SELECT id, quantity FROM orders) SELECT a FROM c",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a CTE column list naming more columns than the CTE outputs is an error",
            sql: "WITH c(a, b) AS (SELECT id FROM orders) SELECT a FROM c",
            expected_findings: &[(
                "E223",
                "CTE column alias count does not match its output column count",
                24,
            )],
        },
    ];
    let schema = typed_duckdb_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::PostgreSQL, &POSTGRES, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

