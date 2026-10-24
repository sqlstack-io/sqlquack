use crate::verify::tests::helpers::round_trip;
use crate::verify::tests::test_types::{RoundTripTestCase, Tampering};

#[test]
fn given_duckdb_stand_in_results_when_verifying_then_only_untampered_results_pass() {
    let test_cases = [
        RoundTripTestCase {
            description: "pending, stand-in recording and replay agree",
            tampering: Tampering::None,
            expected_pending_before: 2,
            expected_pending_after: 0,
            expected_verdicts: vec!["passed", "passed"],
        },
        RoundTripTestCase {
            description: "a changed value with a consistent Arrow hash fails the comparison",
            tampering: Tampering::ValueChanged,
            expected_pending_before: 2,
            expected_pending_after: 0,
            expected_verdicts: vec!["failed", "passed"],
        },
        RoundTripTestCase {
            description: "an Arrow file edited after recording fails the integrity check",
            tampering: Tampering::ArrowEdited,
            expected_pending_before: 2,
            expected_pending_after: 0,
            expected_verdicts: vec!["failed", "passed"],
        },
    ];

    for test_case in test_cases {
        let observed = round_trip(test_case.tampering);
        assert_eq!(
            observed.pending_before, test_case.expected_pending_before,
            "{}",
            test_case.description
        );
        assert_eq!(
            observed.pending_after, test_case.expected_pending_after,
            "{}",
            test_case.description
        );
        assert_eq!(
            observed.verdicts, test_case.expected_verdicts,
            "{}",
            test_case.description
        );
    }
}
