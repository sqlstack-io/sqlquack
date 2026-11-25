use crate::helpers::{converted_first_value, parsed_copies};
use crate::test_types::{CaseSubjectTestCase, ChainValueTestCase};

#[test]
fn given_a_simple_case_over_a_computed_operand_when_converting_then_duckdb_binds_the_operand_once()
{
    let test_cases = [
        CaseSubjectTestCase {
            description: "two WHENs with literal results",
            branches: 2,
            result: "'v{i}'",
            expected_copies: 1,
        },
        CaseSubjectTestCase {
            description: "thirty-two WHENs with literal results",
            branches: 32,
            result: "'v{i}'",
            expected_copies: 1,
        },
        CaseSubjectTestCase {
            description: "thirty-two WHENs with column results",
            branches: 32,
            result: "b",
            expected_copies: 1,
        },
    ];
    for test_case in test_cases {
        let whens: Vec<String> = (0..test_case.branches)
            .map(|i| {
                format!(
                    "WHEN 'W{i}' THEN {}",
                    test_case.result.replace("{i}", &i.to_string())
                )
            })
            .collect();
        let sql = format!(
            "SELECT CASE UPPER(a || 'operand_marker') {} ELSE NULL END AS c \
             FROM (SELECT 'x' AS a, 'y' AS b)",
            whens.join(" ")
        );

        let copies = parsed_copies(&sql, "operand_marker");

        assert_eq!(
            copies, test_case.expected_copies,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_simple_case_with_plain_results_when_converting_then_duckdb_returns_snowflakes_value() {
    let test_cases = [
        ChainValueTestCase {
            description: "the first equal WHEN's literal result",
            sql: "SELECT CASE UPPER(a) WHEN 'X' THEN 'first' WHEN 'X' THEN 'second' END AS c \
                  FROM (SELECT 'x' AS a)",
            expected_value: "first",
        },
        ChainValueTestCase {
            description: "a column result",
            sql: "SELECT CASE UPPER(a) WHEN 'Y' THEN 'no' WHEN 'X' THEN b END AS c \
                  FROM (SELECT 'x' AS a, 'from b' AS b)",
            expected_value: "from b",
        },
        ChainValueTestCase {
            description: "ELSE when no WHEN equals the operand",
            sql: "SELECT CASE UPPER(a) WHEN 'Y' THEN 'no' ELSE b END AS c \
                  FROM (SELECT 'x' AS a, 'else' AS b)",
            expected_value: "else",
        },
        ChainValueTestCase {
            description: "a NULL operand equals no WHEN",
            sql: "SELECT CASE UPPER(a) WHEN 'X' THEN 'no' ELSE 'null operand' END AS c \
                  FROM (SELECT CAST(NULL AS VARCHAR) AS a)",
            expected_value: "null operand",
        },
        ChainValueTestCase {
            description: "a number result cast to the CASE's type only where chosen",
            sql: "SELECT CAST(CASE a + 1 WHEN 2 THEN 1.5 WHEN 3 THEN n END AS VARCHAR) AS c \
                  FROM (SELECT 1 AS a, 7 AS n)",
            expected_value: "1.5",
        },
    ];
    for test_case in test_cases {
        let value = converted_first_value(test_case.sql);

        assert_eq!(value, test_case.expected_value, "{}", test_case.description);
    }
}
