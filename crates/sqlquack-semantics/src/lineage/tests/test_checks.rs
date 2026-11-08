use sqlquack_syntax::Dialect;

use crate::dialects::constants::{DUCKDB, POSTGRES, SNOWFLAKE};
use crate::lineage::tests::helpers::{
    bigquery_findings, found_findings, owned_findings, typed_bigquery_shop_schema,
    typed_duckdb_shop_schema, typed_postgres_shop_schema, typed_shop_schema,
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
        QueryFindingsTestCase {
            description: "a quoted read of a grouped column in another case, which DuckDB matches",
            sql: "SELECT \"STATUS\", COUNT(*) AS n FROM orders GROUP BY status",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a grouped CAST read with its column quoted in another case",
            sql: "SELECT CAST(\"STATUS\" AS VARCHAR) AS s, COUNT(*) AS n FROM orders GROUP BY CAST(status AS VARCHAR)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a CAST grouped with its column quoted, read unquoted",
            sql: "SELECT CAST(status AS VARCHAR) AS s, COUNT(*) AS n FROM orders GROUP BY CAST(\"STATUS\" AS VARCHAR)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a grouped expression read with its qualifier and column in other cases",
            sql: "SELECT upper(O.\"Status\") || 'x' AS s, COUNT(*) AS n FROM orders AS o GROUP BY upper(o.status) || 'x'",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a grouped CAST read unqualified, its column offered by one source only",
            sql: "SELECT CAST(ordered_at AS DATE) AS d, COUNT(*) AS n FROM orders AS o JOIN customers AS c ON o.id = c.id GROUP BY CAST(o.ordered_at AS DATE)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a grouped CAST read unqualified, its column offered by both sources",
            sql: "SELECT CAST(id AS VARCHAR) AS d, COUNT(*) AS n FROM orders AS o JOIN customers AS c ON o.id = c.id GROUP BY CAST(o.id AS VARCHAR)",
            expected_findings: &[
                (
                    "E230",
                    "Column must be grouped or used in an aggregate function",
                    12,
                ),
                (
                    "E221",
                    "Ambiguous unqualified column 'id' found in 2 referenced tables",
                    12,
                ),
            ],
        },
        QueryFindingsTestCase {
            description: "a grouped CAST read in the :: spelling with a comment",
            sql: "SELECT status::VARCHAR /* shown */ AS s, COUNT(*) AS n FROM orders GROUP BY CAST(status AS VARCHAR)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a CAST of another column than the grouped one",
            sql: "SELECT CAST(quantity AS VARCHAR) AS s, COUNT(*) AS n FROM orders GROUP BY CAST(status AS VARCHAR)",
            expected_findings: &[(
                "E230",
                "Column must be grouped or used in an aggregate function",
                12,
            )],
        },
        QueryFindingsTestCase {
            description: "EXCLUDE naming a column quoted in another case, which DuckDB matches",
            sql: "WITH c AS (SELECT o.\"STATUS\", o.id FROM orders AS o) SELECT * EXCLUDE (status) FROM c",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "EXCLUDE naming a column the input lacks",
            sql: "WITH c AS (SELECT o.\"STATUS\", o.id FROM orders AS o) SELECT * EXCLUDE (quantity) FROM c",
            expected_findings: &[(
                "E201",
                "Star modifier references unknown column 'quantity'",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "a qualified grouped column read in another case",
            sql: "SELECT O.Status, COUNT(*) AS n FROM orders AS o GROUP BY o.status",
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
fn given_duckdb_relations_whose_columns_are_known_when_checking_names_then_unknown_names_are_found()
{
    let test_cases = [
        QueryFindingsTestCase {
            description: "an unknown qualifier beside generate_series",
            sql: "SELECT zq.n FROM generate_series(1, 3) AS g(n)",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'zq' referenced by column 'n'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "an unknown column of a renamed generate_series",
            sql: "SELECT g.generate_series FROM generate_series(1, 3) AS g(n)",
            expected_findings: &[("E201", "Unknown column 'generate_series' in table 'g'", 9)],
        },
        QueryFindingsTestCase {
            description: "an unknown qualifier beside a cross-joined unnest",
            sql: "SELECT zq.id, t.flag FROM orders AS o CROSS JOIN unnest([false]) AS t(flag)",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'zq' referenced by column 'id'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "the columns range, generate_series and unnest are named after",
            sql: "SELECT r.range, s.generate_series, u.unnest FROM range(2) AS r, generate_series(1, 2) AS s, unnest([1]) AS u",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an unknown column of a VALUES list every column of which is renamed",
            sql: "SELECT missing, status FROM (VALUES ('o1', 'placed')) AS t(order_id, status)",
            expected_findings: &[("E201", "Unknown column 'missing' in table 't'", 7)],
        },
        QueryFindingsTestCase {
            description: "an unknown column of an aliased CTE, named by its alias",
            sql: "WITH counted AS (SELECT id FROM orders) SELECT missing FROM counted AS c",
            expected_findings: &[("E201", "Unknown column 'missing' in table 'c'", 47)],
        },
        QueryFindingsTestCase {
            description: "an unknown column of an aliased stored table, named by the table",
            sql: "SELECT missing FROM orders AS o",
            expected_findings: &[("E201", "Unknown column 'missing' in table 'orders'", 7)],
        },
        QueryFindingsTestCase {
            description: "an unknown column in a CTE nothing reads, which DuckDB never binds",
            sql: "WITH o AS (SELECT missing FROM orders) SELECT id FROM orders AS o",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an unknown column in a CTE read only by another unread CTE",
            sql: "WITH a AS (SELECT missing FROM orders), b AS (SELECT * FROM a) SELECT id FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an unknown column in a CTE read through another CTE",
            sql: "WITH a AS (SELECT missing FROM orders), b AS (SELECT * FROM a) SELECT * FROM b",
            expected_findings: &[("E201", "Unknown column 'missing' in table 'orders'", 18)],
        },
        QueryFindingsTestCase {
            description: "an unknown column in a CTE read by a subquery",
            sql: "WITH a AS (SELECT missing FROM orders) SELECT id FROM orders WHERE id IN (SELECT id FROM a)",
            expected_findings: &[("E201", "Unknown column 'missing' in table 'orders'", 18)],
        },
        QueryFindingsTestCase {
            description: "a VALUES list renamed in part keeps generated names, so nothing is judged",
            sql: "SELECT col1 FROM (VALUES ('o1', 'placed')) AS t(order_id)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an unknown qualifier in a recursive term",
            sql: "WITH RECURSIVE chain AS (SELECT id, 0 AS depth FROM orders UNION ALL SELECT zq.id, chain.depth + 1 FROM orders JOIN chain ON orders.id = chain.id) SELECT id FROM chain",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'zq' referenced by column 'id'",
                76,
            )],
        },
        QueryFindingsTestCase {
            description: "a column both the table and the recursive reference offer",
            sql: "WITH RECURSIVE chain AS (SELECT id, 0 AS depth FROM orders UNION ALL SELECT id, chain.depth + 1 FROM orders JOIN chain ON orders.id = chain.id) SELECT id FROM chain",
            expected_findings: &[(
                "E221",
                "Ambiguous unqualified column 'id' found in 2 referenced tables",
                76,
            )],
        },
        QueryFindingsTestCase {
            description: "a column the anchor does not output",
            sql: "WITH RECURSIVE chain AS (SELECT id, 0 AS depth FROM orders UNION ALL SELECT orders.id, chain.missing FROM orders JOIN chain ON orders.id = chain.id) SELECT id FROM chain",
            expected_findings: &[("E201", "Unknown column 'missing' in table 'chain'", 93)],
        },
        QueryFindingsTestCase {
            description: "a recursive term reading only names the anchor outputs",
            sql: "WITH RECURSIVE chain AS (SELECT id, 0 AS depth FROM orders UNION ALL SELECT orders.id, chain.depth + 1 FROM orders JOIN chain ON orders.id = chain.id WHERE chain.depth < 3) SELECT id, depth FROM chain",
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
            description: "a timestamp against a date string with a comment after it converts",
            sql: "SELECT cutoff FROM shipments WHERE shipped_at >= '2026-10-03' /* c */ \
                  AND shipped_at < '2026-10-04' -- c\n",
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
            description: "unknown functions are only unknown names, declared ones are never errors",
            sql: "SELECT TO_DATE('2026-01-01') AS d, shop_score(status) AS s, mystery(1, 2) AS m FROM orders",
            expected_findings: &[
                ("E202", "Unknown function 'TO_DATE' for dialect DuckDB", 7),
                ("E202", "Unknown function 'mystery' for dialect DuckDB", 60),
            ],
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
            expected_findings: &[("E202", "Unknown function 'mystery' for dialect DuckDB", 7)],
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
                    "Invalid arity for function 'SUBSTRING': got 1, expected 2, 3",
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
            description: "a call the catalog does not know is an unknown name, not a count error",
            sql: "SELECT order_band(status, 1, 2, 3) AS b FROM orders",
            expected_findings: &[(
                "E202",
                "Unknown function 'order_band' for dialect DuckDB",
                7,
            )],
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

#[test]
fn given_a_postgres_query_when_checking_types_and_names_then_findings_follow_postgresql_18_6() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "an integer WHERE condition (18.6: 42804)",
            sql: "SELECT id FROM orders WHERE quantity",
            expected_findings: &[(
                "E211",
                "WHERE expects a boolean predicate, found integer",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "a string literal WHERE condition converts to BOOLEAN (18.6: accepted)",
            sql: "SELECT id FROM orders WHERE 'true'",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an integer AND operand (18.6: 42804)",
            sql: "SELECT id FROM orders, shipments WHERE quantity AND rush",
            expected_findings: &[(
                "E211",
                "Logical left operand expects boolean, found integer",
                39,
            )],
        },
        QueryFindingsTestCase {
            description: "text compared with an integer column is an error, not a runtime cast (18.6: 42883)",
            sql: "SELECT id FROM orders WHERE status = quantity",
            expected_findings: &[(
                "E217",
                "Incompatible comparison between string and integer",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "an integer compared with a boolean (18.6: 42883)",
            sql: "SELECT id FROM orders, shipments WHERE quantity = rush",
            expected_findings: &[(
                "E217",
                "Incompatible comparison between integer and boolean",
                39,
            )],
        },
        QueryFindingsTestCase {
            description: "UNION of text and integer columns (18.6: 42804)",
            sql: "SELECT status FROM orders UNION SELECT quantity FROM orders",
            expected_findings: &[(
                "E215",
                "UNION column 1 has incompatible types: string vs integer",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "UNION of boolean and integer (18.6: 42804)",
            sql: "SELECT rush FROM shipments UNION SELECT 1",
            expected_findings: &[(
                "E215",
                "UNION column 1 has incompatible types: boolean vs integer",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "a string literal UNION an integer converts (18.6: accepted)",
            sql: "SELECT '5' UNION SELECT quantity FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "integer plus text (18.6: 42883)",
            sql: "SELECT quantity + status FROM orders",
            expected_findings: &[(
                "E212",
                "Arithmetic operation expects numeric-compatible operands, found integer and string",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "interval times and over a number, date plus time, date minus timestamp \
                          and time minus time (18.6: accepted)",
            sql: "SELECT took * 2, took / 2, shipped_on + cutoff, shipped_on - shipped_at, \
                  cutoff - cutoff FROM shipments",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a number over an interval (18.6: 42883)",
            sql: "SELECT 2 / took FROM shipments",
            expected_findings: &[(
                "E212",
                "Arithmetic operation expects numeric-compatible operands, found integer and interval",
                11,
            )],
        },
        QueryFindingsTestCase {
            description: "NULLIF of text and an integer is an error (18.6: 42883)",
            sql: "SELECT nullif(status, quantity) FROM orders",
            expected_findings: &[("E213", "Cannot unify string and integer", 14)],
        },
        QueryFindingsTestCase {
            description: "UPPER of an integer, which PostgreSQL does not cast (18.6: 42883)",
            sql: "SELECT upper(quantity) FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'upper' argument 1 expects String, found integer",
                13,
            )],
        },
        QueryFindingsTestCase {
            description: "a timestamp cast to an integer (18.6: 42846)",
            sql: "SELECT ordered_at::int FROM orders",
            expected_findings: &[("E218", "Cannot cast timestamp to integer", 7)],
        },
        QueryFindingsTestCase {
            description: "two CTEs of one name (18.6: 42712)",
            sql: "WITH c AS (SELECT 1), c AS (SELECT 2) SELECT * FROM c",
            expected_findings: &[("E233", "Duplicate CTE name 'c'", 0)],
        },
        QueryFindingsTestCase {
            description: "an output name inside an ORDER BY expression (18.6: 42703)",
            sql: "SELECT quantity AS q FROM orders ORDER BY q + 1",
            expected_findings: &[("E201", "Unknown column 'q' in table 'orders'", 42)],
        },
        QueryFindingsTestCase {
            description: "a parenthesized output name as the ORDER BY key (18.6: accepted)",
            sql: "SELECT quantity AS q FROM orders ORDER BY (q)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an output name in WHERE (18.6: 42703)",
            sql: "SELECT quantity AS q FROM orders WHERE q > 1",
            expected_findings: &[("E201", "Unknown column 'q' in table 'orders'", 39)],
        },
        QueryFindingsTestCase {
            description: "an output name in ROLLUP (18.6: accepted)",
            sql: "SELECT quantity AS q FROM orders GROUP BY ROLLUP (q)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an output name in HAVING is neither known nor grouped (18.6: 42703)",
            sql: "SELECT quantity AS q FROM orders GROUP BY quantity HAVING q > 1",
            expected_findings: &[
                (
                    "E230",
                    "Column must be grouped or used in an aggregate function",
                    58,
                ),
                ("E201", "Unknown column 'q' in table 'orders'", 58),
            ],
        },
        QueryFindingsTestCase {
            description: "system columns of a stored table (18.6: accepted)",
            sql: "SELECT ctid, xmin FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a system column of two stored tables (18.6: 42702)",
            sql: "SELECT ctid FROM orders, customers",
            expected_findings: &[(
                "E221",
                "Ambiguous unqualified column 'ctid' found in 2 referenced tables",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a derived table has no system columns (18.6: 42703)",
            sql: "SELECT ctid FROM (SELECT id FROM orders) s",
            expected_findings: &[("E201", "Unknown column 'ctid' in table 's'", 7)],
        },
        QueryFindingsTestCase {
            description: "a quoted upper-case system column name (18.6: 42703)",
            sql: "SELECT \"CTID\" FROM orders",
            expected_findings: &[("E201", "Unknown column 'ctid' in table 'orders'", 7)],
        },
        QueryFindingsTestCase {
            description: "a ranking function without OVER (18.6: 42809)",
            sql: "SELECT row_number() AS position FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 0)],
        },
        QueryFindingsTestCase {
            description: "a distribution function without OVER (18.6: 42809)",
            sql: "SELECT quantity, cume_dist() FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 0)],
        },
        QueryFindingsTestCase {
            description: "nth_value without OVER inside an expression (18.6: 42809)",
            sql: "SELECT nth_value(quantity, 2) + 1 FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 17)],
        },
        QueryFindingsTestCase {
            description: "window functions with OVER, plain aggregates and a windowed subquery (18.6: accepted)",
            sql: "SELECT row_number() OVER (), cume_dist() OVER (ORDER BY quantity), max(quantity) OVER (), \
                  (SELECT nth_value(id, 2) OVER (ORDER BY id) FROM orders LIMIT 1) FROM orders",
            expected_findings: &[],
        },
    ];
    let schema = typed_postgres_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::PostgreSQL, &POSTGRES, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_duckdb_calls_when_checking_them_against_the_engine_catalogue_then_unknown_names_and_counts_are_found()
 {
    let test_cases = [
        QueryFindingsTestCase {
            description: "a name DuckDB has no function, macro or special form for",
            sql: "SELECT not_a_function(id) AS v FROM orders",
            expected_findings: &[(
                "E202",
                "Unknown function 'not_a_function' for dialect DuckDB",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "SUBSTRING with one argument, where DuckDB takes two or three",
            sql: "SELECT SUBSTRING(status) AS v FROM orders",
            expected_findings: &[(
                "E203",
                "Invalid arity for function 'SUBSTRING': got 1, expected 2, 3",
                17,
            )],
        },
        QueryFindingsTestCase {
            description: "DATE_TRUNC with one argument",
            sql: "SELECT DATE_TRUNC('day') AS v FROM orders",
            expected_findings: &[(
                "E203",
                "Invalid arity for function 'DATE_TRUNC': got 1, expected 2",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "SUM with two arguments",
            sql: "SELECT SUM(quantity, id) AS v FROM orders",
            expected_findings: &[(
                "E203",
                "Invalid arity for function 'SUM': got 2, expected 1",
                11,
            )],
        },
        QueryFindingsTestCase {
            description: "variadic, extension and parser-transformed calls are known",
            sql: "SELECT concat(status) AS a, ifnull(status, 'x') AS b, try(id) AS c, \
                  json_extract(status, '$.a') AS d, st_astext(st_point(1, 2)) AS e FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "GROUP BY ROLLUP and CUBE are grouping constructs, not calls",
            sql: "SELECT status, COUNT(*) AS n FROM orders GROUP BY ROLLUP (status), CUBE (id)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a schema-qualified call is not judged",
            sql: "SELECT main.not_a_function(id) AS v FROM orders",
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
fn given_a_duckdb_qualified_column_its_relation_lacks_when_checking_then_the_relation_is_named() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "an aliased stored table is named by its table",
            sql: "SELECT s.missing FROM orders AS s",
            expected_findings: &[("E201", "Unknown column 'missing' in table 'orders'", 9)],
        },
        QueryFindingsTestCase {
            description: "an aliased CTE is named by the qualifier as written",
            sql: "WITH weekly AS (SELECT id FROM orders) SELECT S.missing FROM weekly AS s",
            expected_findings: &[("E201", "Unknown column 'missing' in table 'S'", 48)],
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
fn given_a_duckdb_columns_pattern_when_checking_then_one_matching_no_listed_column_is_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "a pattern no column of a listed relation matches",
            sql: "SELECT COLUMNS('^nothing_') FROM orders",
            expected_findings: &[("E201", "COLUMNS pattern matches no input columns", 0)],
        },
        QueryFindingsTestCase {
            description: "a pattern matching part of a column name, as DuckDB searches",
            sql: "SELECT COLUMNS('tatu') FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a pattern over a relation whose columns are not all listed",
            sql: "SELECT COLUMNS('^nothing_') FROM events",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a pattern that does not compile is left to the engine",
            sql: "SELECT COLUMNS('(') FROM orders",
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
fn given_a_snowflake_text_key_joined_to_a_number_key_when_checking_then_only_using_is_unreported() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "a USING key of text and a number, the text filtered to numbers",
            sql: "WITH text_keys AS (SELECT status AS check_key FROM orders \
                  WHERE TRY_TO_NUMBER(status) IS NOT NULL), \
                  wide AS (SELECT id AS check_key FROM customers) \
                  SELECT check_key FROM text_keys AS t INNER JOIN wide AS w USING (check_key)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "the same keys compared in ON",
            sql: "SELECT o.id FROM orders AS o INNER JOIN customers AS c ON o.status = c.id",
            expected_findings: &[(
                "W210",
                "Incompatible comparison between string and integer",
                58,
            )],
        },
        QueryFindingsTestCase {
            description: "the same keys compared in WHERE",
            sql: "SELECT o.id FROM orders AS o, customers AS c WHERE o.status = c.id",
            expected_findings: &[(
                "W210",
                "Incompatible comparison between string and integer",
                51,
            )],
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
fn given_a_bigquery_query_when_checking_it_then_findings_follow_googlesql() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "a TIMESTAMP_TRUNC date part is not a column (analyzer: accepted)",
            sql: "SELECT TIMESTAMP_TRUNC(ordered_at, DAY) AS v FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a DATE_DIFF WEEK(<weekday>) part is not a column (analyzer: accepted)",
            sql: "SELECT DATE_DIFF(ordered_on, ordered_on, WEEK(SUNDAY)) AS v FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "grouping matches a column whatever its case (analyzer: accepted)",
            sql: "SELECT Status, COUNT(*) AS n FROM orders GROUP BY status",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "grouping matches an expression whatever its case (analyzer: accepted)",
            sql: "SELECT LOWER(Status) AS s FROM orders AS o GROUP BY lower(O.status)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a catalog column in its own case matches a lower-case read (analyzer: accepted)",
            sql: "SELECT fullname FROM customers",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a string literal compared with a DATE converts (analyzer: accepted)",
            sql: "SELECT id FROM orders WHERE ordered_on = '2020-01-01'",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a field of an unaliased UNNEST of STRUCTs is a column (analyzer: accepted)",
            sql: "SELECT id FROM UNNEST([STRUCT(1 AS id), STRUCT(2 AS other)])",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a name that is not a field of the first STRUCT (analyzer: rejected)",
            sql: "SELECT missing_column FROM UNNEST([STRUCT(1 AS id)])",
            expected_findings: &[(
                "E201",
                "Unknown column 'missing_column' (not found in any referenced table)",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "an aliased UNNEST may name its whole element (analyzer: accepted)",
            sql: "SELECT u FROM UNNEST([STRUCT(1 AS id)]) AS u",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a column neither grouped nor aggregated (analyzer: rejected)",
            sql: "SELECT status FROM orders GROUP BY id",
            expected_findings: &[(
                "E230",
                "Column must be grouped or used in an aggregate function",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "ROW_NUMBER without OVER (analyzer: rejected)",
            sql: "SELECT ROW_NUMBER() AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
        QueryFindingsTestCase {
            description: "CUME_DIST without OVER (analyzer: rejected)",
            sql: "SELECT quantity, CUME_DIST() FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 17)],
        },
        QueryFindingsTestCase {
            description: "two CTEs under one name (analyzer: rejected)",
            sql: "WITH c AS (SELECT 1 AS a), c AS (SELECT 2 AS a) SELECT a FROM c",
            expected_findings: &[("E233", "Duplicate CTE name 'c'", 55)],
        },
        QueryFindingsTestCase {
            description: "FLOAT64 times STRING (analyzer: rejected)",
            sql: "SELECT ratio * status AS v FROM orders",
            expected_findings: &[(
                "E212",
                "Arithmetic operation expects numeric-compatible operands, found numeric and string",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "ABS of a STRING (analyzer: rejected)",
            sql: "SELECT ABS(status) AS v FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'abs' argument 1 expects Numeric, found string",
                11,
            )],
        },
        QueryFindingsTestCase {
            description: "an INT64 WHERE condition (analyzer: rejected)",
            sql: "SELECT id FROM orders WHERE quantity",
            expected_findings: &[(
                "E211",
                "WHERE expects a boolean predicate, found integer",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "STRING compared with an INT64 column (analyzer: rejected)",
            sql: "SELECT id FROM orders WHERE status = quantity",
            expected_findings: &[(
                "E217",
                "Incompatible comparison between string and integer",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "STRING compared with an integer literal (analyzer: rejected)",
            sql: "SELECT id FROM orders WHERE status = 1",
            expected_findings: &[(
                "E217",
                "Incompatible comparison between string and integer",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "DATE compared with a STRING expression (analyzer: rejected)",
            sql: "SELECT id FROM orders WHERE ordered_on = 'x' || status",
            expected_findings: &[(
                "E217",
                "Incompatible comparison between date and string",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "IF branches of INT64 and STRING (analyzer: rejected)",
            sql: "SELECT IF(rush, id, status) AS v FROM orders",
            expected_findings: &[("E213", "Cannot unify integer and string", 10)],
        },
        QueryFindingsTestCase {
            description: "INT64 plus a string literal (analyzer: rejected)",
            sql: "SELECT id + '1' AS v FROM orders",
            expected_findings: &[(
                "E212",
                "Arithmetic operation expects numeric-compatible operands, found integer and string",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a DATE minus a date-shaped string literal (analyzer: accepted)",
            sql: "SELECT ordered_on - '2020-01-01' AS v FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "INT64 LIKE an ARRAY literal (analyzer: rejected)",
            sql: "SELECT id FROM orders WHERE id LIKE [1, 2]",
            expected_findings: &[(
                "E217",
                "LIKE/ILIKE expects string operands, found integer and array",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "EXTRACT from an INT64 (analyzer: rejected)",
            sql: "SELECT EXTRACT(YEAR FROM id) AS v FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'extract' argument 2 expects a temporal argument, found integer",
                25,
            )],
        },
        QueryFindingsTestCase {
            description: "NUMERIC times an INTERVAL (analyzer: rejected)",
            sql: "SELECT price * INTERVAL 1 DAY AS v FROM orders",
            expected_findings: &[(
                "E212",
                "Arithmetic operation expects numeric-compatible operands, found numeric and interval",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "INT64 times an INTERVAL (analyzer: accepted)",
            sql: "SELECT quantity * INTERVAL 1 DAY AS v FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "DATE_ADD on a TIME (analyzer: rejected)",
            sql: "SELECT DATE_ADD(TIME '10:00:00', INTERVAL 1 DAY) AS v FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'date_add' argument 1 expects a date argument, found time",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "DATE_ADD on a DATE (analyzer: accepted)",
            sql: "SELECT DATE_ADD(ordered_on, INTERVAL 1 DAY) AS v FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "STRING IN integer literals (analyzer: rejected)",
            sql: "SELECT id FROM orders WHERE status IN (1, 2)",
            expected_findings: &[(
                "E217",
                "IN item type integer is incompatible with string",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "a DATE minus a TIMESTAMP (analyzer: rejected)",
            sql: "SELECT ordered_on - ordered_at AS v FROM orders",
            expected_findings: &[(
                "E212",
                "Arithmetic operation expects numeric-compatible operands, found date and timestamp",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "DATE_ADD on a JSON value is one error (analyzer: rejected)",
            sql: "SELECT DATE_ADD(JSON '1', INTERVAL 1 DAY) AS v FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'date_add' argument 1 expects a date argument, found json",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "a DATETIME minus a TIMESTAMP names both types (analyzer: rejected)",
            sql: "SELECT DATETIME '2020-01-01 00:00:00' - ordered_at AS v FROM orders",
            expected_findings: &[(
                "E212",
                "Arithmetic operation expects numeric-compatible operands, found datetime and timestamp",
                40,
            )],
        },
        QueryFindingsTestCase {
            description: "TRIM of BYTES with no characters argument (analyzer: rejected)",
            sql: "SELECT TRIM(b'x') AS v FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'trim' of binary needs the bytes to remove",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "SAFE.DATE_ADD checks its arguments as DATE_ADD does (analyzer: rejected)",
            sql: "SELECT SAFE.DATE_ADD(JSON '1', INTERVAL 1 DAY) AS v FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'date_add' argument 1 expects a date argument, found json",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "IF over JSON and INT64 has no common type (analyzer: rejected)",
            sql: "SELECT IF(rush, JSON '1', 1) AS v FROM orders",
            expected_findings: &[("E213", "Cannot unify json and integer", 10)],
        },
        QueryFindingsTestCase {
            description: "an IF condition of JSON is not BOOL (analyzer: rejected)",
            sql: "SELECT IF(JSON 'true', 1, 2) AS v FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'if' argument 1 expects Boolean, found json",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "IF over JSON and NULL is JSON (analyzer: accepted)",
            sql: "SELECT IF(rush, JSON '1', NULL) AS v FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "DATE_SUB on a STRUCT literal (analyzer: rejected)",
            sql: "SELECT DATE_SUB(STRUCT(1 AS a), INTERVAL 1 DAY) AS v FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'date_sub' argument 1 expects a date argument, found struct",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "TRIM of JSON is one error (analyzer: rejected)",
            sql: "SELECT TRIM(JSON '\"x\"') AS v FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'trim' argument 1 expects a string or bytes argument, found json",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "an INTERVAL count of STRING (analyzer: rejected)",
            sql: "SELECT DATE_ADD(ordered_on, INTERVAL status DAY) AS v FROM orders",
            expected_findings: &[(
                "E213",
                "Interval count expects an integer, found string",
                37,
            )],
        },
        QueryFindingsTestCase {
            description: "an INTERVAL count of a string literal (analyzer: accepted)",
            sql: "SELECT DATE_ADD(ordered_on, INTERVAL '1' DAY) AS v FROM orders",
            expected_findings: &[],
        },
    ];
    let schema = typed_bigquery_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            bigquery_findings(test_case.sql, &schema),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}
