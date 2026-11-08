use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::JSON_FIDELITY;
use crate::semi_structured::main::json_guarded::json_guarded;
use crate::semi_structured::tests::helpers::evaluated;
use crate::semi_structured::tests::test_types::TryGuardedTestCase;

#[test]
fn given_text_duckdb_cannot_read_when_trying_to_parse_json_then_only_unfinishable_text_is_null() {
    let test_cases = [
        TryGuardedTestCase {
            description: "a document cut off inside a nested string",
            text: r#"{"status": {"code": "out_for_de"#,
            expected_null: true,
        },
        TryGuardedTestCase {
            description: "a document cut off after a colon, behind blanks",
            text: r#"  {"summary": "#,
            expected_null: true,
        },
        TryGuardedTestCase {
            description: "an unterminated string holding closing brackets",
            text: r#"{"a":"x}]"#,
            expected_null: true,
        },
        TryGuardedTestCase {
            description: "an HTML page",
            text: "<html><body>503</body></html>",
            expected_null: true,
        },
        TryGuardedTestCase {
            description: "a single-quoted string hiding an opening bracket",
            text: "{'a':'['}",
            expected_null: false,
        },
        TryGuardedTestCase {
            description: "a complete array followed by an unclosed one",
            text: "[1] [",
            expected_null: false,
        },
        TryGuardedTestCase {
            description: "an unclosed object behind a comment",
            text: "{\"a\":1 /* { */",
            expected_null: false,
        },
        TryGuardedTestCase {
            description: "a closer too many",
            text: r#"{"a":1}}"#,
            expected_null: false,
        },
        TryGuardedTestCase {
            description: "plain words",
            text: "Service Unavailable",
            expected_null: false,
        },
    ];
    for test_case in test_cases {
        let literal = format!("'{}'", test_case.text.replace('\'', "''"));
        let sql = json_guarded(&literal, true, &ProvenanceLog::default());
        let (value, error) = evaluated(&format!("CAST({sql} AS VARCHAR)"));
        let raised = error
            .as_deref()
            .is_some_and(|error| error.contains(JSON_FIDELITY));
        assert_eq!(
            (value.is_none() && error.is_none(), raised),
            (test_case.expected_null, !test_case.expected_null),
            "{}: {value:?} {error:?}",
            test_case.description
        );
    }
}
