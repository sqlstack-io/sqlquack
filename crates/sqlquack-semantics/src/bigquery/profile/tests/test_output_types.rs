use crate::bigquery::profile::tests::helpers::bigquery_output_type;
use crate::bigquery::profile::tests::helpers::typed_bigquery_shop_schema;
use crate::bigquery::profile::tests::test_types::BigqueryTypeTestCase;

/// Each expected type is the one GoogleSQL's analyzer gives the same query over the same columns.
#[test]
fn given_bigquery_queries_when_typing_outputs_then_each_has_the_analyzer_type() {
    let test_cases = [
        BigqueryTypeTestCase {
            description: "SUM over INT64 stays INT64",
            sql: "SELECT SUM(quantity) AS v FROM orders",
            expected_type: Some("INT64"),
        },
        BigqueryTypeTestCase {
            description: "AVG over INT64 is FLOAT64",
            sql: "SELECT AVG(quantity) AS v FROM orders",
            expected_type: Some("FLOAT64"),
        },
        BigqueryTypeTestCase {
            description: "AVG over NUMERIC stays NUMERIC",
            sql: "SELECT AVG(price) AS v FROM orders",
            expected_type: Some("NUMERIC"),
        },
        BigqueryTypeTestCase {
            description: "a decimal literal is FLOAT64",
            sql: "SELECT 1.5 AS v",
            expected_type: Some("FLOAT64"),
        },
        BigqueryTypeTestCase {
            description: "an exponent literal is FLOAT64",
            sql: "SELECT 1e3 AS v",
            expected_type: Some("FLOAT64"),
        },
        BigqueryTypeTestCase {
            description: "INT64 divided by INT64 is FLOAT64",
            sql: "SELECT quantity / 2 AS v FROM orders",
            expected_type: Some("FLOAT64"),
        },
        BigqueryTypeTestCase {
            description: "an integer literal takes its NUMERIC partner's type",
            sql: "SELECT price + 1 AS v FROM orders",
            expected_type: Some("NUMERIC"),
        },
        BigqueryTypeTestCase {
            description: "a decimal literal takes its NUMERIC partner's type",
            sql: "SELECT price * 1.5 AS v FROM orders",
            expected_type: Some("NUMERIC"),
        },
        BigqueryTypeTestCase {
            description: "INT64 plus FLOAT64 is FLOAT64",
            sql: "SELECT quantity + ratio AS v FROM orders",
            expected_type: Some("FLOAT64"),
        },
        BigqueryTypeTestCase {
            description: "DATE_DIFF is INT64",
            sql: "SELECT DATE_DIFF(ordered_on, ordered_on, DAY) AS v FROM orders",
            expected_type: Some("INT64"),
        },
        BigqueryTypeTestCase {
            description: "DATE_DIFF with a WEEK(<weekday>) part is INT64",
            sql: "SELECT DATE_DIFF(ordered_on, ordered_on, WEEK(SUNDAY)) AS v FROM orders",
            expected_type: Some("INT64"),
        },
        BigqueryTypeTestCase {
            description: "TIMESTAMP_TRUNC keeps TIMESTAMP",
            sql: "SELECT TIMESTAMP_TRUNC(ordered_at, HOUR) AS v FROM orders",
            expected_type: Some("TIMESTAMP"),
        },
        BigqueryTypeTestCase {
            description: "DATE_TRUNC to a week keeps DATE",
            sql: "SELECT DATE_TRUNC(ordered_on, WEEK(MONDAY)) AS v FROM orders",
            expected_type: Some("DATE"),
        },
        BigqueryTypeTestCase {
            description: "LAST_DAY keeps DATE",
            sql: "SELECT LAST_DAY(ordered_on, MONTH) AS v FROM orders",
            expected_type: Some("DATE"),
        },
        BigqueryTypeTestCase {
            description: "DATE_ADD of an interval keeps DATE",
            sql: "SELECT DATE_ADD(ordered_on, INTERVAL 1 DAY) AS v FROM orders",
            expected_type: Some("DATE"),
        },
        BigqueryTypeTestCase {
            description: "a DATE minus an integer is DATE",
            sql: "SELECT ordered_on - 1 AS v FROM orders",
            expected_type: Some("DATE"),
        },
        BigqueryTypeTestCase {
            description: "DATE of a TIMESTAMP is DATE",
            sql: "SELECT DATE(ordered_at) AS v FROM orders",
            expected_type: Some("DATE"),
        },
        BigqueryTypeTestCase {
            description: "a cast to TIMESTAMP is TIMESTAMP",
            sql: "SELECT CAST('2020-01-01 00:00:00' AS TIMESTAMP) AS v",
            expected_type: Some("TIMESTAMP"),
        },
        BigqueryTypeTestCase {
            description: "EXTRACT is INT64",
            sql: "SELECT EXTRACT(YEAR FROM ordered_on) AS v FROM orders",
            expected_type: Some("INT64"),
        },
        BigqueryTypeTestCase {
            description: "string concatenation is STRING",
            sql: "SELECT status || 'x' AS v FROM orders",
            expected_type: Some("STRING"),
        },
        BigqueryTypeTestCase {
            description: "COUNT(*) is INT64",
            sql: "SELECT COUNT(*) AS v FROM orders",
            expected_type: Some("INT64"),
        },
        BigqueryTypeTestCase {
            description: "ROW_NUMBER is INT64",
            sql: "SELECT ROW_NUMBER() OVER (ORDER BY id) AS v FROM orders",
            expected_type: Some("INT64"),
        },
        BigqueryTypeTestCase {
            description: "IF with a NULL branch takes the other branch's type",
            sql: "SELECT IF(rush, quantity, NULL) AS v FROM orders",
            expected_type: Some("INT64"),
        },
        BigqueryTypeTestCase {
            description: "COALESCE of NUMERIC and an integer literal is NUMERIC",
            sql: "SELECT COALESCE(price, 0) AS v FROM orders",
            expected_type: Some("NUMERIC"),
        },
        BigqueryTypeTestCase {
            description: "SAFE_DIVIDE of integers is FLOAT64",
            sql: "SELECT SAFE_DIVIDE(quantity, 2) AS v FROM orders",
            expected_type: Some("FLOAT64"),
        },
        BigqueryTypeTestCase {
            description: "a NUMERIC branch over an INT64 branch is NUMERIC",
            sql: "SELECT price AS v FROM orders UNION ALL SELECT quantity FROM orders",
            expected_type: Some("NUMERIC"),
        },
        BigqueryTypeTestCase {
            description: "an INT64 branch over a FLOAT64 branch is FLOAT64",
            sql: "SELECT quantity AS v FROM orders UNION ALL SELECT ratio FROM orders",
            expected_type: Some("FLOAT64"),
        },
        BigqueryTypeTestCase {
            description: "a FLOAT64 constant branch over NUMERIC is left untyped, not FLOAT64 (analyzer: NUMERIC)",
            sql: "SELECT price AS v FROM orders UNION ALL SELECT 1.5",
            expected_type: None,
        },
        BigqueryTypeTestCase {
            description: "a catalog column in its own case matches a lower-case read",
            sql: "SELECT fullname AS v FROM customers",
            expected_type: Some("STRING"),
        },
        BigqueryTypeTestCase {
            description: "a cast to STRING(MAX) is STRING",
            sql: "SELECT CAST(id AS STRING(MAX)) AS v FROM orders",
            expected_type: Some("STRING"),
        },
        BigqueryTypeTestCase {
            description: "a cast to BYTES(MAX) is BYTES",
            sql: "SELECT CAST(status AS BYTES(MAX)) AS v FROM orders",
            expected_type: Some("BYTES"),
        },
    ];
    let schema = typed_bigquery_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            bigquery_output_type(&schema, test_case.sql).as_deref(),
            test_case.expected_type,
            "{}",
            test_case.description
        );
    }
}
