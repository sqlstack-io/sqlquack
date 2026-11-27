use crate::macros::tests::helpers::{Outcome, helpers, read, shown};
use crate::macros::tests::test_types::JsonReadTestCase;

#[test]
fn given_text_when_trying_to_read_it_as_json_then_version_2_reads_what_version_1_reads() {
    let fidelity = "json_fidelity#s: text DuckDB does not read as JSON, which Snowflake may read";
    let test_cases = [
        JsonReadTestCase {
            description: "JSON",
            text: r#"{"a":[1,"x"]}"#,
            expected_outcome: Ok(Some(r#"{"a":[1,"x"]}"#)),
        },
        JsonReadTestCase {
            description: "blank text",
            text: "  ",
            expected_outcome: Ok(None),
        },
        JsonReadTestCase {
            description: "an unfinished object",
            text: "{bad",
            expected_outcome: Ok(None),
        },
        JsonReadTestCase {
            description: "an unfinished array",
            text: "[1,",
            expected_outcome: Ok(None),
        },
        JsonReadTestCase {
            description: "a word without brackets, which Snowflake may read",
            text: "abc",
            expected_outcome: Err(fidelity),
        },
        JsonReadTestCase {
            description: "a closed array followed by an open one",
            text: "[][",
            expected_outcome: Err(fidelity),
        },
        JsonReadTestCase {
            description: "markup",
            text: "<p>",
            expected_outcome: Ok(None),
        },
    ];
    let connection = helpers();
    for test_case in test_cases {
        let outcomes: Vec<Outcome> = [
            "sqlquack.json_read_v1('#s', ?, true)",
            "sqlquack.json_read_v2('#s', ?, true)",
            "sqlquack.json_checked_v2('#s', ?, true, []::VARCHAR[], []::VARCHAR[])",
        ]
        .into_iter()
        .map(|call| read(&connection, (call, test_case.text)))
        .collect();

        assert_eq!(
            shown(&outcomes),
            [test_case.expected_outcome; 3],
            "{}",
            test_case.description
        );
    }
}
