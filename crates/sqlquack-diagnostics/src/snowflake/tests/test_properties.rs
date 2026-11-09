use crate::snowflake::tests::helpers::{property_violations, violations_of};
use crate::snowflake::tests::test_types::{ExtremeInputTestCase, PropertyTestCase};

/// Room for linear work on the largest inputs in a debug build, but not for a quadratic path.
const EXTREME_INPUT_LIMIT: std::time::Duration = std::time::Duration::from_secs(10);

#[test]
#[ignore = "timing"]
fn given_generated_sql_when_parsed_and_diagnosed_then_nothing_panics_hangs_or_points_outside() {
    let test_cases = [
        PropertyTestCase {
            description: "fragments of Snowflake SQL from the first seed",
            seed: 0x5eed_0001,
            inputs: 400,
            expected_violations: Vec::new(),
        },
        PropertyTestCase {
            description: "fragments of Snowflake SQL from a second seed",
            seed: 0x5eed_0002,
            inputs: 400,
            expected_violations: Vec::new(),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            property_violations(test_case.seed, test_case.inputs),
            test_case.expected_violations,
            "{}",
            test_case.description
        );
    }
}

#[test]
#[ignore = "timing"]
fn given_extreme_sql_when_parsed_and_diagnosed_then_it_finishes_without_panicking() {
    let test_cases = [
        ExtremeInputTestCase {
            description: "parentheses nested 5,000 deep",
            input: format!("SELECT {}1{}", "(".repeat(5_000), ")".repeat(5_000)),
            expected_violations: Vec::new(),
        },
        ExtremeInputTestCase {
            description: "a 5 MB string literal",
            input: format!("SELECT '{}' AS s", "x".repeat(5 << 20)),
            expected_violations: Vec::new(),
        },
        ExtremeInputTestCase {
            description: "20,000 statements",
            input: "SELECT 1;\n".repeat(20_000),
            expected_violations: Vec::new(),
        },
        ExtremeInputTestCase {
            description: "an unterminated block comment after a long line",
            input: format!("SELECT {} /* never closed", "a + ".repeat(2_000)),
            expected_violations: Vec::new(),
        },
        ExtremeInputTestCase {
            description: "a 1 MB single line that fails to parse",
            input: format!("SELECT {} FROM", "o.order_id, ".repeat(80_000)),
            expected_violations: Vec::new(),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            violations_of(&test_case.input, EXTREME_INPUT_LIMIT),
            test_case.expected_violations,
            "{}",
            test_case.description
        );
    }
}
