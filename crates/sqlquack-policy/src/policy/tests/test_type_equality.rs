use crate::policy::tests::helpers::equality_reason;
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
