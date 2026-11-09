use sqlquack_syntax::Dialect;

use crate::helpers::{located_matches_faithful, located_text};
use crate::test_types::{LocatedAgreementTestCase, LocatedTestCase};

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
