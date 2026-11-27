use crate::policy::tests::helpers::{databricks_equality_reason, equality_reason};
use crate::policy::tests::test_types::TypeEqualityTestCase;

#[test]
fn given_result_types_when_matching_then_only_declared_equalities_admit_them() {
    let test_cases = [
        TypeEqualityTestCase {
            description: "the same precision and scale",
            snowflake: "NUMBER(10,2)",
            duckdb: "DECIMAL(10,2)",
            expected_reason: Some("same values"),
        },
        TypeEqualityTestCase {
            description: "a different scale is not admitted",
            snowflake: "NUMBER(10,2)",
            duckdb: "DECIMAL(10,3)",
            expected_reason: None,
        },
        TypeEqualityTestCase {
            description: "an undeclared pair is not admitted",
            snowflake: "NUMBER(38,0)",
            duckdb: "BIGINT",
            expected_reason: None,
        },
        TypeEqualityTestCase {
            description: "a bound parameter on one side only",
            snowflake: "VARCHAR(16777216)",
            duckdb: "VARCHAR",
            expected_reason: Some("length does not change values"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            equality_reason(test_case.snowflake, test_case.duckdb).as_deref(),
            test_case.expected_reason,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_databricks_array_types_when_matching_then_only_the_rendered_element_types_admit_text() {
    let test_cases = [
        TypeEqualityTestCase {
            description: "an array of integers compares by its text",
            snowflake: "ARRAY<INT>",
            duckdb: "VARCHAR",
            expected_reason: Some("Databricks's text"),
        },
        TypeEqualityTestCase {
            description: "an array of doubles, whose text the engines write differently",
            snowflake: "ARRAY<DOUBLE>",
            duckdb: "VARCHAR",
            expected_reason: None,
        },
        TypeEqualityTestCase {
            description: "an array never compares with a DuckDB list",
            snowflake: "ARRAY<INT>",
            duckdb: "INTEGER[]",
            expected_reason: None,
        },
    ];

    for test_case in test_cases {
        let reason = databricks_equality_reason(test_case.snowflake, test_case.duckdb);
        assert_eq!(
            reason.as_deref().map(|reason| test_case
                .expected_reason
                .is_some_and(|expected| reason.contains(expected))),
            test_case.expected_reason.map(|_| true),
            "{}",
            test_case.description
        );
    }
}
