use crate::verify::tests::helpers::{arrow_edited, round_trip, untouched, value_changed};
use crate::verify::tests::test_types::{RoundTrip, RoundTripTestCase};

#[test]
fn given_duckdb_stand_in_results_when_verifying_then_only_untampered_results_pass() {
    let test_cases = [
        RoundTripTestCase {
            description: "pending, stand-in recording and replay agree",
            tampering: untouched,
            expected_round_trip: RoundTrip {
                pending_before: 2,
                pending_after: 0,
                verdicts: vec!["passed".to_owned(), "passed".to_owned()],
            },
        },
        RoundTripTestCase {
            description: "a changed value with a consistent Arrow hash fails the comparison",
            tampering: value_changed,
            expected_round_trip: RoundTrip {
                pending_before: 2,
                pending_after: 0,
                verdicts: vec!["failed".to_owned(), "passed".to_owned()],
            },
        },
        RoundTripTestCase {
            description: "an Arrow file edited after recording fails the integrity check",
            tampering: arrow_edited,
            expected_round_trip: RoundTrip {
                pending_before: 2,
                pending_after: 0,
                verdicts: vec!["failed".to_owned(), "passed".to_owned()],
            },
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            round_trip(test_case.tampering),
            test_case.expected_round_trip,
            "{}",
            test_case.description
        );
    }
}
