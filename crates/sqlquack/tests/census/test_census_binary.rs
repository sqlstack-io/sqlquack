use crate::helpers::{run_on_events, run_on_refused_upstream, run_on_shop};
use crate::test_types::{
    BinaryOutcome, BinaryTestCase, SourceVariantOutcome, SourceVariantTestCase, UpstreamOutcome,
    UpstreamTestCase,
};

#[test]
fn given_sql_directory_when_running_census_binary_then_reports_on_the_right_streams() {
    let test_cases = [
        BinaryTestCase {
            description: "JSON goes to stdout; the slowest files and the completion message to stderr",
            arguments: &["census", "--json"],
            expected_outcome: BinaryOutcome {
                exit_code: Some(0),
                files: Some(2),
                unresolved_name: Some(2),
                completion_reported: true,
                slowest_reported: true,
                names_printed: false,
            },
        },
        BinaryTestCase {
            description: "text output names no file, model or relation",
            arguments: &["census"],
            expected_outcome: BinaryOutcome {
                exit_code: Some(0),
                files: None,
                unresolved_name: None,
                completion_reported: true,
                slowest_reported: true,
                names_printed: false,
            },
        },
        BinaryTestCase {
            description: "--names prints file, model and relation names",
            arguments: &["census", "--json", "--names"],
            expected_outcome: BinaryOutcome {
                exit_code: Some(0),
                files: Some(2),
                unresolved_name: Some(2),
                completion_reported: true,
                slowest_reported: true,
                names_printed: true,
            },
        },
        BinaryTestCase {
            description: "an unknown dialect is a usage error",
            arguments: &["census", "--dialect", "unknown"],
            expected_outcome: BinaryOutcome {
                exit_code: Some(2),
                files: None,
                unresolved_name: None,
                completion_reported: false,
                slowest_reported: false,
                names_printed: false,
            },
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            run_on_shop(test_case.arguments),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_model_comparing_a_source_variant_when_running_census_then_source_variant_json_is_counted()
 {
    let test_cases = [
        SourceVariantTestCase {
            description: "a source VARIANT compared by text converts, relying on source_variant_json",
            payload: "[\"PAYLOAD\", \"VARIANT\"]",
            flags: &[],
            expected_outcome: SourceVariantOutcome {
                exit_code: Some(0),
                convertible: Some(1),
                labelled: Some(1),
            },
        },
        SourceVariantTestCase {
            description: "strict mode refuses the comparison relying on source_variant_json",
            payload: "[\"PAYLOAD\", \"VARIANT\"]",
            flags: &["--strict"],
            expected_outcome: SourceVariantOutcome {
                exit_code: Some(0),
                convertible: Some(0),
                labelled: None,
            },
        },
        SourceVariantTestCase {
            description: "a schema column marked typed_leaves is unproven, so the comparison is refused",
            payload: "{\"name\": \"PAYLOAD\", \"type\": \"VARIANT\", \"typed_leaves\": true}",
            flags: &[],
            expected_outcome: SourceVariantOutcome {
                exit_code: Some(0),
                convertible: Some(0),
                labelled: None,
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            run_on_events(test_case.payload, test_case.flags),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_refused_upstream_model_when_running_census_binary_then_strict_and_unit_testable_differ()
{
    let test_cases = [
        UpstreamTestCase {
            description: "the schema file types the refused model: its reader is unit-testable, \
                          labelled, and not strict",
            typed: true,
            flags: &["--json", "--names"],
            expected_outcome: UpstreamOutcome {
                exit_code: Some(0),
                unit_testable: Some(1),
                strict: Some(0),
                labels: vec!["upstream_refused:fuzzy_names".to_owned()],
                text_counts: None,
                plan_bounds: Some([1, 0]),
            },
        },
        UpstreamTestCase {
            description: "without names the label numbers the refused model",
            typed: true,
            flags: &["--json"],
            expected_outcome: UpstreamOutcome {
                exit_code: Some(0),
                unit_testable: Some(1),
                strict: Some(0),
                labels: vec!["upstream_refused:#1".to_owned()],
                text_counts: None,
                plan_bounds: Some([1, 0]),
            },
        },
        UpstreamTestCase {
            description: "no known columns for the refused model: its reader stays blocked",
            typed: false,
            flags: &["--json", "--names"],
            expected_outcome: UpstreamOutcome {
                exit_code: Some(0),
                unit_testable: Some(0),
                strict: Some(0),
                labels: Vec::new(),
                text_counts: None,
                plan_bounds: Some([0, 0]),
            },
        },
        UpstreamTestCase {
            description: "text output names both counts",
            typed: true,
            flags: &[],
            expected_outcome: UpstreamOutcome {
                exit_code: Some(0),
                unit_testable: None,
                strict: None,
                labels: Vec::new(),
                text_counts: Some(
                    "convertible: strict 0 (every model upstream converts too), unit-testable 1 \
                     (the file's own SQL converts, reading the schema file's columns for 1 \
                     upstream-refused)"
                        .to_owned(),
                ),
                plan_bounds: None,
            },
        },
        UpstreamTestCase {
            description: "assuming the refused model's construct supported bounds both counts",
            typed: false,
            flags: &["--json", "--assume-supported", "fn:SOUNDEX"],
            expected_outcome: UpstreamOutcome {
                exit_code: Some(0),
                unit_testable: Some(0),
                strict: Some(0),
                labels: Vec::new(),
                text_counts: None,
                plan_bounds: Some([2, 2]),
            },
        },
        UpstreamTestCase {
            description: "an unknown rule in --assume-supported is a usage error",
            typed: false,
            flags: &["--json", "--assume-supported", "rule.unknown"],
            expected_outcome: UpstreamOutcome {
                exit_code: Some(2),
                unit_testable: None,
                strict: None,
                labels: Vec::new(),
                text_counts: None,
                plan_bounds: None,
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            run_on_refused_upstream(test_case.typed, test_case.flags),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
