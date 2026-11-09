use crate::verify::tests::helpers::{
    TAMPERED_CANONICAL, TAMPERED_CASE, TAMPERED_TYPING, arrow_edited, defaults_with_date_format,
    errored, pinned_value_changed, recorded_before_pinning, round_trip, round_trip_with_defaults,
    untouched, value_changed, values_dropped,
};
use crate::verify::tests::test_types::{RoundTrip, RoundTripTestCase, SessionTestCase};

#[test]
fn given_duckdb_stand_in_results_when_verifying_then_only_untampered_results_pass() {
    let test_cases = [
        RoundTripTestCase {
            description: "pending, stand-in recording and replay agree",
            tampered: TAMPERED_CASE,
            tampering: untouched,
            expected_round_trip: RoundTrip {
                pending_before: 5,
                pending_after: 0,
                unproven_sessions: 0,
                verdicts: vec![
                    "passed".to_owned(),
                    "passed".to_owned(),
                    "typed".to_owned(),
                    "passed".to_owned(),
                ],
            },
        },
        RoundTripTestCase {
            description: "a changed value with a consistent Arrow hash fails the comparison",
            tampered: TAMPERED_CASE,
            tampering: value_changed,
            expected_round_trip: RoundTrip {
                pending_before: 5,
                pending_after: 0,
                unproven_sessions: 0,
                verdicts: vec![
                    "failed".to_owned(),
                    "passed".to_owned(),
                    "typed".to_owned(),
                    "passed".to_owned(),
                ],
            },
        },
        RoundTripTestCase {
            description: "a conversion case recorded without values fails",
            tampered: TAMPERED_CASE,
            tampering: values_dropped,
            expected_round_trip: RoundTrip {
                pending_before: 5,
                pending_after: 0,
                unproven_sessions: 0,
                verdicts: vec![
                    "failed".to_owned(),
                    "passed".to_owned(),
                    "typed".to_owned(),
                    "passed".to_owned(),
                ],
            },
        },
        RoundTripTestCase {
            description: "a typing case recorded as a warehouse error fails",
            tampered: TAMPERED_TYPING,
            tampering: errored,
            expected_round_trip: RoundTrip {
                pending_before: 5,
                pending_after: 0,
                unproven_sessions: 0,
                verdicts: vec![
                    "passed".to_owned(),
                    "passed".to_owned(),
                    "failed".to_owned(),
                    "passed".to_owned(),
                ],
            },
        },
        RoundTripTestCase {
            description: "an Arrow file edited after recording fails the integrity check",
            tampered: TAMPERED_CASE,
            tampering: arrow_edited,
            expected_round_trip: RoundTrip {
                pending_before: 5,
                pending_after: 0,
                unproven_sessions: 0,
                verdicts: vec![
                    "failed".to_owned(),
                    "passed".to_owned(),
                    "typed".to_owned(),
                    "passed".to_owned(),
                ],
            },
        },
        RoundTripTestCase {
            description: "an equivalence whose canonical spelling recorded another value fails",
            tampered: TAMPERED_CANONICAL,
            tampering: value_changed,
            expected_round_trip: RoundTrip {
                pending_before: 5,
                pending_after: 0,
                unproven_sessions: 0,
                verdicts: vec![
                    "passed".to_owned(),
                    "passed".to_owned(),
                    "typed".to_owned(),
                    "failed".to_owned(),
                ],
            },
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            round_trip(test_case.tampering, test_case.tampered),
            test_case.expected_round_trip,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_recorded_session_parameters_when_verifying_then_only_the_profile_s_values_pass() {
    let round_trip_of = |first: &str, unproven_sessions: usize| RoundTrip {
        pending_before: 5,
        pending_after: 0,
        unproven_sessions,
        verdicts: vec![
            first.to_owned(),
            "passed".to_owned(),
            "typed".to_owned(),
            "passed".to_owned(),
        ],
    };
    let test_cases = [
        SessionTestCase {
            description: "a pinned parameter recorded with another value fails",
            tampering: pinned_value_changed,
            defaults: None,
            expected_round_trip: round_trip_of("failed", 0),
        },
        SessionTestCase {
            description: "a result recorded before pinning without recorded defaults is unproven",
            tampering: recorded_before_pinning,
            defaults: None,
            expected_round_trip: round_trip_of("passed", 1),
        },
        SessionTestCase {
            description: "a result recorded before pinning passes when the account defaults match",
            tampering: recorded_before_pinning,
            defaults: Some(defaults_with_date_format("YYYY-MM-DD")),
            expected_round_trip: round_trip_of("passed", 0),
        },
        SessionTestCase {
            description: "a result recorded before pinning fails when an account default differs",
            tampering: recorded_before_pinning,
            defaults: Some(defaults_with_date_format("DD/MM/YYYY")),
            expected_round_trip: round_trip_of("failed", 0),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            round_trip_with_defaults((test_case.tampering, TAMPERED_CASE), test_case.defaults),
            test_case.expected_round_trip,
            "{}",
            test_case.description
        );
    }
}
