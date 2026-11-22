use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{located_matches_faithful, located_text, located_text_in};
use crate::test_types::{LocatedAgreementTestCase, LocatedTestCase, LocationStabilityTestCase};

#[test]
fn given_a_located_parse_when_reading_a_node_then_its_span_covers_its_source() {
    let test_cases = [
        LocatedTestCase {
            description: "the statement covers the whole query",
            sql: "SELECT CAST(o.amount AS INT) + 1 AS total, UPPER(name) FROM orders o WHERE status IN ('open', 'held') AND NOT (qty > 2)",
            pointer: "/0",
            expected_text: Ok("SELECT CAST(o.amount AS INT) + 1 AS total, UPPER(name) FROM orders o WHERE status IN ('open', 'held') AND NOT (qty > 2)"),
        },
        LocatedTestCase {
            description: "a cast covers its call",
            sql: "SELECT CAST(o.amount AS INT) + 1 AS total, UPPER(name) FROM orders o WHERE status IN ('open', 'held') AND NOT (qty > 2)",
            pointer: "/0/select/expressions/0/alias/this/add/left",
            expected_text: Ok("CAST(o.amount AS INT)"),
        },
        LocatedTestCase {
            description: "an addition covers both operands",
            sql: "SELECT CAST(o.amount AS INT) + 1 AS total, UPPER(name) FROM orders o WHERE status IN ('open', 'held') AND NOT (qty > 2)",
            pointer: "/0/select/expressions/0/alias/this",
            expected_text: Ok("CAST(o.amount AS INT) + 1"),
        },
        LocatedTestCase {
            description: "an alias covers its expression and name",
            sql: "SELECT CAST(o.amount AS INT) + 1 AS total, UPPER(name) FROM orders o WHERE status IN ('open', 'held') AND NOT (qty > 2)",
            pointer: "/0/select/expressions/0",
            expected_text: Ok("CAST(o.amount AS INT) + 1 AS total"),
        },
        LocatedTestCase {
            description: "a call covers its name and arguments",
            sql: "SELECT CAST(o.amount AS INT) + 1 AS total, UPPER(name) FROM orders o WHERE status IN ('open', 'held') AND NOT (qty > 2)",
            pointer: "/0/select/expressions/1",
            expected_text: Ok("UPPER(name)"),
        },
        LocatedTestCase {
            description: "IN covers its list",
            sql: "SELECT CAST(o.amount AS INT) + 1 AS total, UPPER(name) FROM orders o WHERE status IN ('open', 'held') AND NOT (qty > 2)",
            pointer: "/0/select/where_clause/this/and/left",
            expected_text: Ok("status IN ('open', 'held')"),
        },
        LocatedTestCase {
            description: "NOT covers its operand",
            sql: "SELECT CAST(o.amount AS INT) + 1 AS total, UPPER(name) FROM orders o WHERE status IN ('open', 'held') AND NOT (qty > 2)",
            pointer: "/0/select/where_clause/this/and/right",
            expected_text: Ok("NOT (qty > 2)"),
        },
        LocatedTestCase {
            description: "a parenthesised comparison covers its parentheses",
            sql: "SELECT CAST(o.amount AS INT) + 1 AS total, UPPER(name) FROM orders o WHERE status IN ('open', 'held') AND NOT (qty > 2)",
            pointer: "/0/select/where_clause/this/and/right/not/this",
            expected_text: Ok("(qty > 2)"),
        },
        LocatedTestCase {
            description: "a CTE body covers its query",
            sql: "WITH recent AS (SELECT id FROM orders) SELECT id FROM (SELECT id FROM recent) AS r",
            pointer: "/0/select/with/ctes/0/this",
            expected_text: Ok("SELECT id FROM orders"),
        },
        LocatedTestCase {
            description: "a derived table covers its parentheses",
            sql: "WITH recent AS (SELECT id FROM orders) SELECT id FROM (SELECT id FROM recent) AS r",
            pointer: "/0/select/from/expressions/0",
            expected_text: Ok("(SELECT id FROM recent) AS r"),
        },
        LocatedTestCase {
            description: "a keyword alias of an ASOF JOIN table covers its name",
            sql: "SELECT * FROM x ASOF JOIN y OFFSET MATCH_CONDITION (x.t >= OFFSET.t)",
            pointer: "/0/select/joins/0/this/alias/alias",
            expected_text: Ok("OFFSET"),
        },
        LocatedTestCase {
            description: "the first branch of a set operation covers its own query",
            sql: "SELECT id FROM orders UNION ALL SELECT id FROM returns",
            pointer: "/0/union/left",
            expected_text: Ok("SELECT id FROM orders"),
        },
        LocatedTestCase {
            description: "the last branch of a set operation covers its own query",
            sql: "SELECT id FROM orders UNION ALL SELECT id FROM returns",
            pointer: "/0/union/right",
            expected_text: Ok("SELECT id FROM returns"),
        },
        LocatedTestCase {
            description: "a middle branch covers its own query",
            sql: "SELECT 1 AS a EXCEPT SELECT 2 EXCEPT SELECT 3",
            pointer: "/0/except/left/except/right",
            expected_text: Ok("SELECT 2"),
        },
        LocatedTestCase {
            description: "a parenthesised branch covers its parentheses",
            sql: "SELECT a FROM x UNION ALL (SELECT b FROM y)",
            pointer: "/0/union/right",
            expected_text: Ok("(SELECT b FROM y)"),
        },
        LocatedTestCase {
            description: "the query inside a parenthesised branch covers itself",
            sql: "SELECT a FROM x UNION ALL (SELECT b FROM y)",
            pointer: "/0/union/right/subquery/this",
            expected_text: Ok("SELECT b FROM y"),
        },
        LocatedTestCase {
            description: "a set operation's ORDER BY stays outside its last branch",
            sql: "SELECT 1 AS k UNION ALL SELECT 2 ORDER BY k",
            pointer: "/0/union/right",
            expected_text: Ok("SELECT 2"),
        },
        LocatedTestCase {
            description: "a nested set operation covers its two branches",
            sql: "SELECT 1 AS a EXCEPT SELECT 2 EXCEPT SELECT 3",
            pointer: "/0/except/left",
            expected_text: Ok("SELECT 1 AS a EXCEPT SELECT 2"),
        },
        LocatedTestCase {
            description: "a set operation covers its trailing ORDER BY",
            sql: "SELECT 1 AS k UNION ALL SELECT 2 ORDER BY k",
            pointer: "/0/union",
            expected_text: Ok("SELECT 1 AS k UNION ALL SELECT 2 ORDER BY k"),
        },
        LocatedTestCase {
            description: "INTERSECT covers only the branches it binds",
            sql: "SELECT 1 AS a UNION SELECT 2 INTERSECT SELECT 3",
            pointer: "/0/union/right",
            expected_text: Ok("SELECT 2 INTERSECT SELECT 3"),
        },
        LocatedTestCase {
            description: "UNION covers the INTERSECT bound to its right",
            sql: "SELECT 1 AS a UNION SELECT 2 INTERSECT SELECT 3",
            pointer: "/0/union",
            expected_text: Ok("SELECT 1 AS a UNION SELECT 2 INTERSECT SELECT 3"),
        },
        LocatedTestCase {
            description: "a query covers its WITH clause",
            sql: "WITH RECURSIVE n AS (SELECT 1 AS i UNION ALL SELECT i + 1 FROM n WHERE i < 10) SELECT i FROM n",
            pointer: "/0",
            expected_text: Ok("WITH RECURSIVE n AS (SELECT 1 AS i UNION ALL SELECT i + 1 FROM n WHERE i < 10) SELECT i FROM n"),
        },
        LocatedTestCase {
            description: "a set operation covers its WITH clause",
            sql: "WITH a AS (SELECT 1 AS x) SELECT x FROM a UNION SELECT 2",
            pointer: "/0",
            expected_text: Ok("WITH a AS (SELECT 1 AS x) SELECT x FROM a UNION SELECT 2"),
        },
        LocatedTestCase {
            description: "INTERSECT covers its WITH clause",
            sql: "WITH a AS (SELECT 1 AS x) SELECT x FROM a INTERSECT SELECT 2",
            pointer: "/0",
            expected_text: Ok("WITH a AS (SELECT 1 AS x) SELECT x FROM a INTERSECT SELECT 2"),
        },
        LocatedTestCase {
            description: "a parenthesised query after WITH covers both",
            sql: "WITH a AS (SELECT 1 AS x) (SELECT x FROM a)",
            pointer: "/0",
            expected_text: Ok("WITH a AS (SELECT 1 AS x) (SELECT x FROM a)"),
        },
        LocatedTestCase {
            description: "a CTE body covers its own WITH clause",
            sql: "WITH a AS (WITH b AS (SELECT 1 AS x) SELECT x FROM b) SELECT x FROM a",
            pointer: "/0/select/with/ctes/0/this",
            expected_text: Ok("WITH b AS (SELECT 1 AS x) SELECT x FROM b"),
        },
        LocatedTestCase {
            description: "a derived query covers its WITH clause",
            sql: "SELECT s.x FROM (WITH b AS (SELECT 1 AS x) SELECT x FROM b) AS s",
            pointer: "/0/select/from/expressions/0/subquery/this",
            expected_text: Ok("WITH b AS (SELECT 1 AS x) SELECT x FROM b"),
        },
        LocatedTestCase {
            description: "a query covers the hint before it",
            sql: "/*+ some hint */ SELECT b.foo FROM baz AS b",
            pointer: "/0",
            expected_text: Ok("/*+ some hint */ SELECT b.foo FROM baz AS b"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            located_text(test_case.sql, test_case.pointer),
            test_case
                .expected_text
                .map(str::to_owned)
                .map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_any_sql_when_parsed_located_then_it_equals_the_faithful_parse() {
    let test_cases = [
        LocatedAgreementTestCase {
            description: "a query with calls, casts and predicates",
            sql: "SELECT CAST(o.amount AS INT) + 1 AS total, UPPER(name) FROM orders o WHERE status IN ('open', 'held') AND NOT (qty > 2)",
            dialect: Dialect::Snowflake,
            expected_same: true,
        },
        LocatedAgreementTestCase {
            description: "nested queries",
            sql: "WITH recent AS (SELECT id FROM orders) SELECT id FROM (SELECT id FROM recent) AS r",
            dialect: Dialect::Snowflake,
            expected_same: true,
        },
        LocatedAgreementTestCase {
            description: "set operations with trailing modifiers",
            sql: "SELECT 1 AS a UNION SELECT 2 INTERSECT (SELECT 3) ORDER BY a LIMIT 2",
            dialect: Dialect::Snowflake,
            expected_same: true,
        },
        LocatedAgreementTestCase {
            description: "a DuckDB query",
            sql: "SELECT a::INT + 1 FROM t WHERE b BETWEEN 1 AND 2",
            dialect: Dialect::DuckDB,
            expected_same: true,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            located_matches_faithful(test_case.sql, test_case.dialect),
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_parse_for_locations_when_reading_queries_then_it_records_no_range_of_its_own_for_them() {
    let test_cases = [
        LocationStabilityTestCase {
            description: "a statement's range starts at its WITH",
            sql: "WITH a AS (SELECT 1 AS x) SELECT x FROM a",
            pointer: "/0",
            expected_text: Ok("WITH a AS (SELECT 1 AS x) SELECT x FROM a"),
        },
        LocationStabilityTestCase {
            description: "a statement's range starts at its hint",
            sql: "/*+ some hint */ SELECT 1",
            pointer: "/0",
            expected_text: Ok("/*+ some hint */ SELECT 1"),
        },
        LocationStabilityTestCase {
            description: "a pivot's input keeps no range, so the pivot is placed as before",
            sql: "SELECT * FROM (SELECT channel, total FROM orders) PIVOT (SUM(total) FOR channel IN ('web'))",
            pointer: "/0/select/from/expressions/0/pivot/this/subquery/this",
            expected_text: Err("the node has no span"),
        },
        LocationStabilityTestCase {
            description: "a set operation's branch keeps none either",
            sql: "SELECT id FROM orders UNION ALL SELECT id FROM returns",
            pointer: "/0/union/right",
            expected_text: Err("the node has no span"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            located_text_in(test_case.sql, test_case.pointer, CallMode::FaithfulLocated),
            test_case
                .expected_text
                .map(str::to_owned)
                .map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
