use crate::helpers::{run_on_events, run_on_shop};
use crate::test_types::{
    BinaryOutcome, BinaryTestCase, SourceVariantOutcome, SourceVariantTestCase,
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
