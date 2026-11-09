use sqlquack_syntax::Dialect;

use crate::dialects::constants::DUCKDB;
use crate::lineage::tests::helpers::{found_findings, owned_findings, typed_duckdb_shop_schema};
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
                  NTH_VALUE(id, 1) OVER (ORDER BY id) AS n FROM orders",
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
        QueryFindingsTestCase {
            description: "a ranking function without OVER (1.4: Binder Error)",
            sql: "SELECT id, row_number() AS position FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 0)],
        },
        QueryFindingsTestCase {
            description: "an offset function without OVER inside an expression (1.4: Binder Error)",
            sql: "SELECT lag(quantity) + 1 AS previous FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 11)],
        },
        QueryFindingsTestCase {
            description: "window functions with OVER and the first and last aggregates (1.4: accepted)",
            sql: "SELECT row_number() OVER (), lag(quantity) OVER (ORDER BY id), first(id), last(id) \
                  FROM orders GROUP BY ALL",
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
