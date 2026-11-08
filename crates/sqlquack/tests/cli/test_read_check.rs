use crate::helpers::converted_result_over;
use crate::test_types::ReadCheckTestCase;

/// The text of the read check's raise for a key repeated in one object.
const REPEATED_KEY: &str = "JSON with a key repeated in one object";

#[test]
fn given_a_key_repeated_outside_the_paths_read_when_running_then_only_reads_reaching_it_raise() {
    let sources = r#"sources:
  - name: events
    columns:
      - name: payload
        type: VARIANT
"#;
    let setup = r#"CREATE TABLE "EVENTS" ("PAYLOAD" VARCHAR);
        INSERT INTO "EVENTS" VALUES ('{"customer":{"id":"C1"},"notes":{"x":1,"x":2}}')"#;
    let test_cases = [
        ReadCheckTestCase {
            description: "a path that never reaches the repeated key returns Snowflake's value",
            sql: "SELECT payload:customer.id::VARCHAR AS c FROM events",
            expected_value: Some("C1"),
            expected_repeat_raised: false,
        },
        ReadCheckTestCase {
            description: "a path to the object holding the repeated key raises",
            sql: "SELECT payload:notes::VARCHAR AS c FROM events",
            expected_value: None,
            expected_repeat_raised: true,
        },
        ReadCheckTestCase {
            description: "the whole value, read without a path, raises",
            sql: "SELECT payload::VARCHAR AS c FROM events",
            expected_value: None,
            expected_repeat_raised: true,
        },
    ];
    for test_case in test_cases {
        let result = converted_result_over(test_case.sql, (sources, setup));
        assert_eq!(
            (
                result.as_deref().ok(),
                result
                    .as_ref()
                    .is_err_and(|error| error.contains(REPEATED_KEY))
            ),
            (test_case.expected_value, test_case.expected_repeat_raised),
            "{}: {result:?}",
            test_case.description
        );
    }
}
