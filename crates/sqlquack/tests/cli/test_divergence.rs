use crate::helpers::verify_overflow_case;
use crate::test_types::{DivergenceOutcome, DivergenceTestCase};

const MARKED: &str = "expected_divergence = \"numeric_range\"\n";

#[test]
fn given_a_recorded_overflow_when_verifying_then_only_a_declared_expected_divergence_is_accepted() {
    let test_cases = [
        DivergenceTestCase {
            description: "a case marking the divergence the conversion relies on diverges as expected",
            case_marker: MARKED,
            policy_default: "warn",
            expected_outcome: DivergenceOutcome {
                exit_code: Some(0),
                verdict: "diverged".to_owned(),
            },
        },
        DivergenceTestCase {
            description: "the same recording without the marker fails: DuckDB raised, Snowflake did not",
            case_marker: "",
            policy_default: "warn",
            expected_outcome: DivergenceOutcome {
                exit_code: Some(1),
                verdict: "failed".to_owned(),
            },
        },
        DivergenceTestCase {
            description: "a policy refusing the divergence by default makes the conversion refuse",
            case_marker: MARKED,
            policy_default: "refuse",
            expected_outcome: DivergenceOutcome {
                exit_code: Some(0),
                verdict: "refused".to_owned(),
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            verify_overflow_case(test_case.case_marker, test_case.policy_default),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
