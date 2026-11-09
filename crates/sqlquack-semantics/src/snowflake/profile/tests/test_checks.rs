use sqlquack_syntax::Dialect;

use crate::dialects::constants::SNOWFLAKE;
use crate::lineage::tests::helpers::{found_findings, owned_findings, typed_shop_schema};
use crate::lineage::tests::test_types::QueryFindingsTestCase;

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
