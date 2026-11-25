use crate::macros::tests::helpers::{Outcome, calls_in_definition, helpers, shown, written};
use crate::macros::tests::test_types::{EscapeCallsTestCase, VariantTextTestCase};

#[test]
fn given_variant_values_when_writing_them_then_version_2_writes_what_version_1_writes() {
    let test_cases = [
        VariantTextTestCase {
            description: "SQL NULL",
            value: None,
            expected_text: Ok(None),
            expected_json_text: Ok(None),
        },
        VariantTextTestCase {
            description: "JSON null",
            value: Some("null"),
            expected_text: Ok(None),
            expected_json_text: Ok(Some("null")),
        },
        VariantTextTestCase {
            description: "a boolean",
            value: Some("true"),
            expected_text: Ok(Some("true")),
            expected_json_text: Ok(Some("true")),
        },
        VariantTextTestCase {
            description: "a negative integer",
            value: Some("-12"),
            expected_text: Ok(Some("-12")),
            expected_json_text: Ok(Some("-12")),
        },
        VariantTextTestCase {
            description: "a number with an exponent",
            value: Some("1e5"),
            expected_text: Ok(Some("1e5")),
            expected_json_text: Ok(Some("1e5")),
        },
        VariantTextTestCase {
            description: "a string with escapes",
            value: Some(r#""tab\tq\"x""#),
            expected_text: Ok(Some("tab\tq\"x")),
            expected_json_text: Ok(Some(r#""tab\tq\"x""#)),
        },
        VariantTextTestCase {
            description: "a string with control characters",
            value: Some(r#""\u001F\u007F""#),
            expected_text: Ok(Some("\u{1f}\u{7f}")),
            expected_json_text: Ok(Some(r#""\u001f\u007F""#)),
        },
        VariantTextTestCase {
            description: "an empty object",
            value: Some("{}"),
            expected_text: Ok(Some("{}")),
            expected_json_text: Ok(Some("{}")),
        },
        VariantTextTestCase {
            description: "an array of a number and a string",
            value: Some(r#"[1,2.5,"x"]"#),
            expected_text: Ok(Some(r#"[1,2.5,"x"]"#)),
            expected_json_text: Ok(Some(r#"[1,2.5,"x"]"#)),
        },
        VariantTextTestCase {
            description: "an array holding an escaped control character",
            value: Some(r#"["\u001F"]"#),
            expected_text: Ok(Some(r#"["\u001f"]"#)),
            expected_json_text: Ok(Some(r#"["\u001f"]"#)),
        },
        VariantTextTestCase {
            description: "an object whose keys are in byte order",
            value: Some(r#"{"a":1,"b":[true,null]}"#),
            expected_text: Ok(Some(r#"{"a":1,"b":[true,null]}"#)),
            expected_json_text: Ok(Some(r#"{"a":1,"b":[true,null]}"#)),
        },
        VariantTextTestCase {
            description: "an object whose keys are not",
            value: Some(r#"{"b":1,"a":2}"#),
            expected_text: Ok(Some(r#"{"a":2,"b":1}"#)),
            expected_json_text: Ok(Some(r#"{"a":2,"b":1}"#)),
        },
        VariantTextTestCase {
            description: "unsorted keys at every depth, beside an escape",
            value: Some(r#"{"b":{"d":1,"c":[{"z":0,"y":"\u001B"}]},"a":null}"#),
            expected_text: Ok(Some(r#"{"a":null,"b":{"c":[{"y":"\u001b","z":0}],"d":1}}"#)),
            expected_json_text: Ok(Some(r#"{"a":null,"b":{"c":[{"y":"\u001b","z":0}],"d":1}}"#)),
        },
        VariantTextTestCase {
            description: "unsorted keys DuckDB could sort only by rewriting a number",
            value: Some(r#"{"b":1.50,"a":2}"#),
            expected_text: Err(
                "json_fidelity#s: an object whose keys DuckDB could sort only by rewriting a number",
            ),
            expected_json_text: Err(
                "json_fidelity#s: an object whose keys DuckDB could sort only by rewriting a number",
            ),
        },
    ];
    let connection = helpers();
    for test_case in test_cases {
        let texts: Vec<Outcome> = ["variant_text_v1", "variant_text_v2"]
            .into_iter()
            .map(|name| written(&connection, (name, test_case.value)))
            .collect();
        let json_texts: Vec<Outcome> = ["variant_json_text_v1", "variant_json_text_v2"]
            .into_iter()
            .map(|name| written(&connection, (name, test_case.value)))
            .collect();

        assert_eq!(
            shown(&texts),
            [test_case.expected_text; 2],
            "{}",
            test_case.description
        );
        assert_eq!(
            shown(&json_texts),
            [test_case.expected_json_text; 2],
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_the_variant_text_helpers_when_defined_then_version_2_calls_the_escape_once() {
    let test_cases = [
        EscapeCallsTestCase {
            description: "variant_text_v1 escapes each text it may write",
            name: "variant_text_v1",
            expected_calls: 2,
        },
        EscapeCallsTestCase {
            description: "variant_text_v2 escapes the text it writes once",
            name: "variant_text_v2",
            expected_calls: 1,
        },
        EscapeCallsTestCase {
            description: "variant_json_text_v1 escapes each text it may write",
            name: "variant_json_text_v1",
            expected_calls: 2,
        },
        EscapeCallsTestCase {
            description: "variant_json_text_v2 escapes the text it writes once",
            name: "variant_json_text_v2",
            expected_calls: 1,
        },
    ];
    for test_case in test_cases {
        let calls = calls_in_definition(test_case.name, "json_escaped_v1");

        assert_eq!(
            calls,
            Some(test_case.expected_calls),
            "{}",
            test_case.description
        );
    }
}
