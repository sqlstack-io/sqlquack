use crate::helpers::verify_coverage;
use crate::test_types::{CoverageOutcome, CoverageTestCase};

const UNRECORDED_KEY: &str = "fn:GET[0] argument 1 text: unrecorded_key";

#[test]
fn given_recorded_cases_when_verifying_coverage_then_uncovered_combinations_and_hazards_are_reported()
 {
    let test_cases = [
        CoverageTestCase {
            description: "the recorded key column covers text keys and one hazard, reported only",
            key_families: "[\"text\", \"integer\"]",
            arguments: &[],
            expected_outcome: CoverageOutcome {
                exit_code: Some(0),
                shape_gaps: vec![],
                hazard_gaps: vec![UNRECORDED_KEY.to_owned()],
            },
        },
        CoverageTestCase {
            description: "a declared BOOLEAN key no recorded case uses is an uncovered combination",
            key_families: "[\"text\", \"integer\", \"boolean\"]",
            arguments: &[],
            expected_outcome: CoverageOutcome {
                exit_code: Some(0),
                shape_gaps: vec!["fn:GET(variant, boolean:value)".to_owned()],
                hazard_gaps: vec![UNRECORDED_KEY.to_owned()],
            },
        },
        CoverageTestCase {
            description: "the hard gate fails on the same gaps",
            key_families: "[\"text\", \"integer\"]",
            arguments: &["--require-coverage"],
            expected_outcome: CoverageOutcome {
                exit_code: Some(1),
                shape_gaps: vec![],
                hazard_gaps: vec![UNRECORDED_KEY.to_owned()],
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            verify_coverage(test_case.key_families, test_case.arguments),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
