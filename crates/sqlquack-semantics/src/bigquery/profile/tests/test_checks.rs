use crate::bigquery::profile::tests::helpers::bigquery_findings;
use crate::bigquery::profile::tests::helpers::typed_bigquery_shop_schema;
use crate::lineage::tests::helpers::owned_findings;
use crate::lineage::tests::test_types::QueryFindingsTestCase;

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
