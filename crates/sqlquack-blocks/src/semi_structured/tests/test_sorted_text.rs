use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::JSON_FIDELITY;
use crate::semi_structured::main::json_serialized::json_serialized;
use crate::semi_structured::tests::helpers::{
    evaluated, nested_document, sorted_document, written_sql,
};
use crate::semi_structured::tests::test_types::{GrowthTestCase, SortedTextTestCase};

#[test]
fn given_nested_documents_when_writing_text_then_sql_grows_at_most_linearly_with_depth() {
    let test_cases = [
        GrowthTestCase {
            description: "PARSE_JSON then TO_JSON of a document literal",
            length: |depth| written_sql(&nested_document(depth)).len(),
            expected_writer_only: false,
        },
        GrowthTestCase {
            description: "the writer's SQL beyond the value it writes",
            length: |depth| {
                let literal = format!("'{}'", nested_document(depth));
                json_serialized(&literal, &ProvenanceLog::default()).len() - literal.len()
            },
            expected_writer_only: true,
        },
    ];
    for test_case in test_cases {
        let [one, eight, sixteen] = [1, 8, 16].map(test_case.length);
        assert_eq!(
            (sixteen - eight) * 7,
            (eight - one) * 8,
            "{}: each level must add the same SQL: {one}, {eight}, {sixteen}",
            test_case.description
        );
        assert_eq!(
            sixteen == one,
            test_case.expected_writer_only,
            "{}: the writer adds no SQL per level",
            test_case.description
        );
    }
}

#[test]
fn given_documents_when_writing_text_then_keys_are_sorted_at_every_depth() {
    let test_cases = [
        SortedTextTestCase {
            description: "two keys",
            document: nested_document(1),
            expected_text: Some(sorted_document(1)),
            expected_raise: false,
        },
        SortedTextTestCase {
            description: "sixteen levels",
            document: nested_document(16),
            expected_text: Some(sorted_document(16)),
            expected_raise: false,
        },
        SortedTextTestCase {
            description: "sixty-four levels, beyond any recording, still exact",
            document: nested_document(64),
            expected_text: Some(sorted_document(64)),
            expected_raise: false,
        },
        SortedTextTestCase {
            description: "array order kept, objects inside sorted",
            document: r#"[3,1,{"d":[2,1],"c":"x"}]"#.to_owned(),
            expected_text: Some(r#"[3,1,{"c":"x","d":[2,1]}]"#.to_owned()),
            expected_raise: false,
        },
        SortedTextTestCase {
            description: "recorded escapes",
            document: r#"{"b":"\u0001\u001f\u007f\t\n\"\\/","a":null}"#.to_owned(),
            expected_text: Some(r#"{"a":null,"b":"\u0001\u001f\u007f\t\n\"\\/"}"#.to_owned()),
            expected_raise: false,
        },
        SortedTextTestCase {
            description: "every control character takes Snowflake's escape, hex in lower case",
            document: r#"{"\u001b":"\u0000\b\f\r\u000b\u001e","a":"\\u001F"}"#.to_owned(),
            expected_text: Some(
                r#"{"\u001b":"\u0000\b\f\r\u000b\u001e","a":"\\u001F"}"#.to_owned(),
            ),
            expected_raise: false,
        },
        SortedTextTestCase {
            description: "C1 controls, U+2029 and U+FEFF stay raw, keys in byte order",
            document: r#"{"\ufeff":"\u2029","\u0085":"\u009f","b":"\u0080"}"#.to_owned(),
            expected_text: Some(
                "{\"b\":\"\u{80}\",\"\u{85}\":\"\u{9f}\",\"\u{feff}\":\"\u{2029}\"}".to_owned(),
            ),
            expected_raise: false,
        },
    ];
    for test_case in test_cases {
        let (text, error) = evaluated(&written_sql(&test_case.document));
        assert_eq!(text, test_case.expected_text, "{}", test_case.description);
        assert_eq!(
            error.is_some_and(|message| message.contains(JSON_FIDELITY)),
            test_case.expected_raise,
            "{}",
            test_case.description
        );
    }
}
