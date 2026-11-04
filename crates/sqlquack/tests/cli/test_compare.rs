use crate::helpers::compare_outcome;
use crate::test_types::CompareTestCase;

#[test]
fn given_converted_sql_and_a_recording_when_compared_then_each_outcome_is_judged_strictly() {
    let test_cases = [
        CompareTestCase {
            description: "the same names, types and rows in order match",
            sql: "SELECT CAST(i AS DECIMAL(38, 0)) AS \"I\", x AS \"X\" FROM fruit ORDER BY i",
            ordered: true,
            arrow_sha256: None,
            expected_outcome: "matches",
        },
        CompareTestCase {
            description: "one different value differs",
            sql: "SELECT CAST(i AS DECIMAL(38, 0)) AS \"I\", \
                  CASE WHEN i = 1 THEN 'kiwi' ELSE x END AS \"X\" FROM fruit ORDER BY i",
            ordered: true,
            arrow_sha256: None,
            expected_outcome: "differs",
        },
        CompareTestCase {
            description: "rows in another order differ when the order is compared",
            sql: "SELECT CAST(i AS DECIMAL(38, 0)) AS \"I\", x AS \"X\" FROM fruit ORDER BY i DESC",
            ordered: true,
            arrow_sha256: None,
            expected_outcome: "differs",
        },
        CompareTestCase {
            description: "rows in another order match as a bag",
            sql: "SELECT CAST(i AS DECIMAL(38, 0)) AS \"I\", x AS \"X\" FROM fruit ORDER BY i DESC",
            ordered: false,
            arrow_sha256: None,
            expected_outcome: "matches",
        },
        CompareTestCase {
            description: "SQL DuckDB cannot run is a DuckDB error",
            sql: "SELECT i AS \"I\", x AS \"X\" FROM missing_fruit",
            ordered: false,
            arrow_sha256: None,
            expected_outcome: "duckdb_error",
        },
        CompareTestCase {
            description: "an Arrow file whose hash differs from its sidecar is unreadable",
            sql: "SELECT CAST(i AS DECIMAL(38, 0)) AS \"I\", x AS \"X\" FROM fruit ORDER BY i",
            ordered: true,
            arrow_sha256: Some("0000000000000000000000000000000000000000000000000000000000000000"),
            expected_outcome: "unreadable",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            compare_outcome(&test_case),
            format!("\"fruit\" \"{}\"", test_case.expected_outcome),
            "{}",
            test_case.description
        );
    }
}
