use crate::helpers::{
    census_warnings, compiled_dbt_counts, converted_files, cursor_model_convertible,
    default_census, run_on_declared_orders, run_on_events, run_on_placed_orders,
    run_on_refused_upstream, run_on_shop,
};
use crate::test_types::{
    BinaryOutcome, BinaryTestCase, CompiledDbtTestCase, ConvertedDirTestCase, ConvertedFile,
    CursorTestCase, DeclaredOutcome, DeclaredTestCase, DefaultOutcome, DefaultOutputTestCase,
    SessionTimeZoneTestCase, SourceVariantOutcome, SourceVariantTestCase, UpstreamOutcome,
    UpstreamTestCase, WarningsTestCase,
};

#[test]
fn given_sql_directory_when_running_census_binary_then_reports_on_the_right_streams() {
    let test_cases = [
        BinaryTestCase {
            description: "--anonymous JSON goes to stdout and names nothing; the completion message to stderr",
            arguments: &["census", "--anonymous", "--json", "--no-config"],
            expected_outcome: BinaryOutcome {
                exit_code: Some(1),
                files: Some(2),
                unresolved_name: Some(2),
                completion_reported: true,
                slowest_reported: false,
                names_printed: false,
            },
        },
        BinaryTestCase {
            description: "--assume-supported without --chain-models is a usage error",
            arguments: &[
                "census",
                "--detailed",
                "--json",
                "--assume-supported",
                "fn:SOUNDEX",
                "--no-config",
            ],
            expected_outcome: BinaryOutcome {
                exit_code: Some(2),
                files: None,
                unresolved_name: None,
                completion_reported: false,
                slowest_reported: false,
                names_printed: false,
            },
        },
        BinaryTestCase {
            description: "--assume-supported without --detailed or --anonymous is a usage error",
            arguments: &[
                "census",
                "--json",
                "--chain-models",
                "--assume-supported",
                "fn:SOUNDEX",
                "--no-config",
            ],
            expected_outcome: BinaryOutcome {
                exit_code: Some(2),
                files: None,
                unresolved_name: None,
                completion_reported: false,
                slowest_reported: false,
                names_printed: false,
            },
        },
        BinaryTestCase {
            description: "--anonymous text names no file, model or relation",
            arguments: &["census", "--anonymous", "--no-config"],
            expected_outcome: BinaryOutcome {
                exit_code: Some(1),
                files: None,
                unresolved_name: None,
                completion_reported: true,
                slowest_reported: false,
                names_printed: false,
            },
        },
        BinaryTestCase {
            description: "--detailed prints file, model and relation names, and the slowest files",
            arguments: &["census", "--detailed", "--json", "--no-config"],
            expected_outcome: BinaryOutcome {
                exit_code: Some(1),
                files: Some(2),
                unresolved_name: Some(2),
                completion_reported: true,
                slowest_reported: true,
                names_printed: true,
            },
        },
        BinaryTestCase {
            description: "--anonymous wins over --detailed: the slowest files, but no names",
            arguments: &[
                "census",
                "--detailed",
                "--anonymous",
                "--json",
                "--no-config",
            ],
            expected_outcome: BinaryOutcome {
                exit_code: Some(1),
                files: Some(2),
                unresolved_name: Some(2),
                completion_reported: true,
                slowest_reported: true,
                names_printed: false,
            },
        },
        BinaryTestCase {
            description: "without a policy file census runs under the built-in policy",
            arguments: &["census", "--anonymous", "--json"],
            expected_outcome: BinaryOutcome {
                exit_code: Some(1),
                files: Some(2),
                unresolved_name: Some(2),
                completion_reported: true,
                slowest_reported: false,
                names_printed: false,
            },
        },
        BinaryTestCase {
            description: "a missing project folder is a usage error, as for convert",
            arguments: &[
                "census",
                "--detailed",
                "--no-config",
                "--project",
                "/nonexistent/project",
            ],
            expected_outcome: BinaryOutcome {
                exit_code: Some(2),
                files: None,
                unresolved_name: None,
                completion_reported: false,
                slowest_reported: false,
                names_printed: false,
            },
        },
        BinaryTestCase {
            description: "an unreadable SQL path is a usage error, as for convert",
            arguments: &["census", "--detailed", "--no-config", "/nonexistent/models"],
            expected_outcome: BinaryOutcome {
                exit_code: Some(2),
                files: None,
                unresolved_name: None,
                completion_reported: false,
                slowest_reported: false,
                names_printed: false,
            },
        },
        BinaryTestCase {
            description: "an unknown dialect is a usage error",
            arguments: &[
                "census",
                "--detailed",
                "--dialect",
                "unknown",
                "--no-config",
            ],
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
            payload: "      - name: payload\n        type: VARIANT\n",
            flags: &["--anonymous"],
            expected_outcome: SourceVariantOutcome {
                exit_code: Some(0),
                convertible: Some(1),
                labelled: Some(1),
            },
        },
        SourceVariantTestCase {
            description: "strict mode refuses the comparison relying on source_variant_json",
            payload: "      - name: payload\n        type: VARIANT\n",
            flags: &["--anonymous", "--strict"],
            expected_outcome: SourceVariantOutcome {
                exit_code: Some(1),
                convertible: Some(0),
                labelled: None,
            },
        },
        SourceVariantTestCase {
            description: "a source column marked typed_leaves is unproven, so the comparison is refused",
            payload: "      - name: payload\n        type: VARIANT\n        typed_leaves: true\n",
            flags: &["--anonymous"],
            expected_outcome: SourceVariantOutcome {
                exit_code: Some(1),
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
            description: "a source file types the refused model: its reader is unit-testable, \
                          labelled, and not strict",
            typed: true,
            flags: &["--json"],
            expected_outcome: UpstreamOutcome {
                exit_code: Some(1),
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
            flags: &["--anonymous", "--json"],
            expected_outcome: UpstreamOutcome {
                exit_code: Some(1),
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
            flags: &["--json"],
            expected_outcome: UpstreamOutcome {
                exit_code: Some(1),
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
            flags: &["--anonymous"],
            expected_outcome: UpstreamOutcome {
                exit_code: Some(1),
                unit_testable: None,
                strict: None,
                labels: Vec::new(),
                text_counts: Some(
                    "convertible: strict 0 (every model upstream converts too), unit-testable 1 \
                     (the file's own SQL converts; 1 with an upstream model refused or not \
                     scanned)"
                        .to_owned(),
                ),
                plan_bounds: None,
            },
        },
        UpstreamTestCase {
            description: "assuming the refused model's construct supported bounds both counts",
            typed: false,
            flags: &["--anonymous", "--json", "--assume-supported", "fn:SOUNDEX"],
            expected_outcome: UpstreamOutcome {
                exit_code: Some(1),
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
            flags: &[
                "--anonymous",
                "--json",
                "--assume-supported",
                "rule.unknown",
            ],
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

#[test]
fn given_a_model_reading_a_date_as_zoned_when_running_census_then_it_relies_on_a_utc_session() {
    let test_cases = [
        SessionTimeZoneTestCase {
            description: "a DATE read as TIMESTAMP_TZ converts, relying on session_timezone_utc",
            flags: &["--anonymous"],
            expected_outcome: SourceVariantOutcome {
                exit_code: Some(0),
                convertible: Some(1),
                labelled: Some(1),
            },
        },
        SessionTimeZoneTestCase {
            description: "strict mode converts it in a UTC session, which strict mode counts as a fact",
            flags: &["--anonymous", "--strict"],
            expected_outcome: SourceVariantOutcome {
                exit_code: Some(0),
                convertible: Some(1),
                labelled: Some(1),
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            run_on_placed_orders(test_case.flags),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_assumed_keys_matching_nothing_when_running_census_then_each_is_warned_about() {
    let test_cases = [WarningsTestCase {
        description: "an empty key and a construct no refusal names each warn; a matching one \
                      does not",
        flags: &[
            "--anonymous",
            "--assume-supported",
            "fn:SOUNDEX,,fn:EDITDISTANCE",
        ],
        expected_warnings: vec![
            "sqlquack census: warning: --assume-supported has an empty key, which is ignored"
                .to_owned(),
            "sqlquack census: warning: --assume-supported 'fn:EDITDISTANCE' matches no \
             collected refusal, so it changes no bound"
                .to_owned(),
        ],
    }];
    for test_case in test_cases {
        assert_eq!(
            census_warnings(test_case.flags),
            test_case.expected_warnings,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_cursor_intrinsics_when_running_census_then_only_a_known_cursor_type_types_them() {
    let test_cases = [
        CursorTestCase {
            description: "without a cursor type the intrinsics stay unknown, so the model is refused",
            flags: &["--anonymous"],
            expected_convertible: Some(0),
        },
        CursorTestCase {
            description: "an integer cursor's bounds are integers, so the model converts",
            flags: &["--anonymous", "--cursor-type", "integer"],
            expected_convertible: Some(1),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            cursor_model_convertible(test_case.flags),
            test_case.expected_convertible,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_declared_offset_when_running_census_then_names_appear_only_under_names() {
    let test_cases = [
        DeclaredTestCase {
            description: "with --anonymous the assumption is counted by its policy id alone",
            declare_model: false,
            flags: &["--anonymous"],
            expected_outcome: DeclaredOutcome {
                exit_code: Some(0),
                assumptions: vec!["declared_offset".to_owned()],
                divergences: vec!["declared_offset at fn:TO_DATE".to_owned()],
                model_declaration_refused: false,
            },
        },
        DeclaredTestCase {
            description: "with --detailed the declared column is named",
            declare_model: false,
            flags: &[],
            expected_outcome: DeclaredOutcome {
                exit_code: Some(0),
                assumptions: vec!["declared_offset:ORDERS.PLACED_AT".to_owned()],
                divergences: vec!["declared_offset:ORDERS.PLACED_AT at fn:TO_DATE".to_owned()],
                model_declaration_refused: false,
            },
        },
        DeclaredTestCase {
            description: "an offset declared on a scanned model's output is refused",
            declare_model: true,
            flags: &["--anonymous"],
            expected_outcome: DeclaredOutcome {
                exit_code: Some(2),
                assumptions: Vec::new(),
                divergences: Vec::new(),
                model_declaration_refused: true,
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            run_on_declared_orders(test_case.declare_model, test_case.flags),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_converted_dir_set_when_running_census_then_each_converted_model_sql_is_written() {
    let test_cases = [
        ConvertedDirTestCase {
            description: "the converted model's DuckDB SQL is written; the refused model's is not",
            set_variable: true,
            expected_files: vec![ConvertedFile {
                model: "customer_ids.sql".to_owned(),
                holds_select: true,
            }],
        },
        ConvertedDirTestCase {
            description: "without the variable nothing is written",
            set_variable: false,
            expected_files: Vec::new(),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            converted_files(test_case.set_variable),
            test_case.expected_files,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_compiled_dbt_models_with_qualified_names_when_running_census_then_they_convert() {
    let test_cases = [
        CompiledDbtTestCase {
            description: "the staging model reads its declared source by database.schema.table; \
                          the mart's model relation is unknown without chaining",
            flags: &["--anonymous"],
            expected_counts: (Some(1), Some(1)),
        },
        CompiledDbtTestCase {
            description: "chained, the mart reads the staging model by its qualified name",
            flags: &["--anonymous", "--chain-models"],
            expected_counts: (Some(2), Some(2)),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            compiled_dbt_counts(test_case.flags),
            test_case.expected_counts,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_models_when_running_census_by_default_then_each_blocked_file_gets_a_diagnostic() {
    let lines =
        |lines: &[&str]| -> Vec<String> { lines.iter().map(|line| (*line).to_owned()).collect() };
    let test_cases = [
        DefaultOutputTestCase {
            description: "every file converts: one summary line and exit 0",
            typed: false,
            files: 1,
            flags: &[],
            expected_outcome: DefaultOutcome {
                exit_code: Some(0),
                text_lines: lines(&["Converted 1 of 1 files (100%)"]),
                json_ok: Some(true),
                json_codes: Vec::new(),
                json_upstream: Vec::new(),
            },
        },
        DefaultOutputTestCase {
            description: "the refused model's reader has no known columns for it: it is blocked \
                          too, but only the model's own refusal counts as a cause",
            typed: false,
            files: 3,
            flags: &[],
            expected_outcome: DefaultOutcome {
                exit_code: Some(1),
                text_lines: lines(&[
                    "error[SQ0002]: SQLQuack can't convert SOUNDEX yet",
                    " --> models/fuzzy_names.sql:1:21",
                    "error[SQ0004]: SQLQuack doesn't know this name",
                    " --> models/name_report.sql:1:48",
                    "  = note: reads `fuzzy_names`, which doesn't convert",
                    "Converted 1 of 3 files (33%)",
                    "Blocked: 2 files, 1 cause (SQ0002 SOUNDEX)",
                ]),
                json_ok: Some(false),
                json_codes: lines(&["SQ0002", "SQ0004"]),
                json_upstream: Vec::new(),
            },
        },
        DefaultOutputTestCase {
            description: "a relation that is neither a model nor declared is a cause, named",
            typed: false,
            files: 4,
            flags: &[],
            expected_outcome: DefaultOutcome {
                exit_code: Some(1),
                text_lines: lines(&[
                    "error[SQ0002]: SQLQuack can't convert SOUNDEX yet",
                    " --> models/fuzzy_names.sql:1:21",
                    "error[SQ0004]: SQLQuack doesn't know this name",
                    " --> models/name_report.sql:1:48",
                    "  = note: reads `fuzzy_names`, which doesn't convert",
                    "error[SQ0004]: SQLQuack doesn't know this name",
                    " --> models/order_report.sql:1:22",
                    "Converted 1 of 4 files (25%)",
                    "Blocked: 3 files, 2 causes (SQ0002 SOUNDEX, SQ0004 ORDERS_SUMMARY)",
                ]),
                json_ok: Some(false),
                json_codes: lines(&["SQ0002", "SQ0004", "SQ0004"]),
                json_upstream: Vec::new(),
            },
        },
        DefaultOutputTestCase {
            description: "the sources type the refused model: chained, its reader converts with a \
                          warning",
            typed: true,
            files: 3,
            flags: &["--chain-models"],
            expected_outcome: DefaultOutcome {
                exit_code: Some(1),
                text_lines: lines(&[
                    "error[SQ0002]: SQLQuack can't convert SOUNDEX yet",
                    " --> models/fuzzy_names.sql:1:21",
                    "warning: models/name_report.sql converts, but reads fuzzy_names, which doesn't",
                    " --> models/name_report.sql",
                    "Converted 2 of 3 files (66%)",
                    "Blocked: 1 file, 1 cause (SQ0002 SOUNDEX)",
                    "Converts with a blocked upstream model: 1",
                ]),
                json_ok: Some(false),
                json_codes: lines(&["SQ0002"]),
                json_upstream: lines(&["models/name_report.sql"]),
            },
        },
        DefaultOutputTestCase {
            description: "the sources type the refused model: without chaining, no upstream warning",
            typed: true,
            files: 3,
            flags: &[],
            expected_outcome: DefaultOutcome {
                exit_code: Some(1),
                text_lines: lines(&[
                    "error[SQ0002]: SQLQuack can't convert SOUNDEX yet",
                    " --> models/fuzzy_names.sql:1:21",
                    "Converted 2 of 3 files (66%)",
                    "Blocked: 1 file, 1 cause (SQ0002 SOUNDEX)",
                ]),
                json_ok: Some(false),
                json_codes: lines(&["SQ0002"]),
                json_upstream: Vec::new(),
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            default_census(test_case.typed, test_case.files, test_case.flags),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
