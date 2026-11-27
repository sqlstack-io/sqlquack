use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::macros::constants::JSON_CHECKED_V2;
use crate::semi_structured::main::json_guarded::json_guarded;
use crate::semi_structured::tests::helpers::evaluated;
use crate::semi_structured::tests::test_types::FoldedLiteralTestCase;

#[test]
fn given_json_literals_when_guarding_then_only_text_the_fast_check_clears_folds_and_reads_the_same()
{
    let test_cases = [
        FoldedLiteralTestCase {
            description: "an object of plain values",
            text: r#" {"b": [1, -2.5, "x"], "a": {"c": null, "d": true}} "#,
            expected_folded: true,
        },
        FoldedLiteralTestCase {
            description: "18 digits, the widest whole number cleared",
            text: "[999999999999999999, -100000000000000000]",
            expected_folded: true,
        },
        FoldedLiteralTestCase {
            description: "escapes the fast check clears, a quote inside the SQL literal",
            text: r#"{"k'": "\"\/\b\f\n\r\t\u00e9\uD7FF"}"#,
            expected_folded: true,
        },
        FoldedLiteralTestCase {
            description: "a key repeated in a nested object",
            text: r#"{"a": {"k": 1, "k": 2}}"#,
            expected_folded: false,
        },
        FoldedLiteralTestCase {
            description: "the same key spelled with an escape",
            text: r#"{"k": 1, "\u006b": 2}"#,
            expected_folded: false,
        },
        FoldedLiteralTestCase {
            description: "19 digits, past 64 bits' safe range",
            text: "[9223372036854775808]",
            expected_folded: false,
        },
        FoldedLiteralTestCase {
            description: "minus zero",
            text: "-0",
            expected_folded: false,
        },
        FoldedLiteralTestCase {
            description: "a zero fraction",
            text: "[1.0]",
            expected_folded: false,
        },
        FoldedLiteralTestCase {
            description: "an exponent",
            text: "[1e2]",
            expected_folded: false,
        },
        FoldedLiteralTestCase {
            description: "NaN",
            text: r#"{"v": NaN}"#,
            expected_folded: false,
        },
        FoldedLiteralTestCase {
            description: "a trailing comma",
            text: "[1, 2,]",
            expected_folded: false,
        },
        FoldedLiteralTestCase {
            description: "an unclosed object",
            text: r#"{"a": 1"#,
            expected_folded: false,
        },
        FoldedLiteralTestCase {
            description: "two documents",
            text: "[1] [2]",
            expected_folded: false,
        },
        FoldedLiteralTestCase {
            description: "an escaped backslash",
            text: r#"["\\"]"#,
            expected_folded: false,
        },
        FoldedLiteralTestCase {
            description: "a UTF-16 surrogate escape",
            text: r#"["\ud83d\ude00"]"#,
            expected_folded: false,
        },
        FoldedLiteralTestCase {
            description: "blank text",
            text: "  ",
            expected_folded: false,
        },
    ];
    let log = ProvenanceLog::default();
    for test_case in test_cases {
        let literal = format!("'{}'", test_case.text.replace('\'', "''"));
        let folded = json_guarded(&literal, false, None, &log);
        let checked = json_guarded(&format!("CAST({literal} AS VARCHAR)"), false, None, &log);
        assert_eq!(
            !folded.contains(JSON_CHECKED_V2),
            test_case.expected_folded,
            "{}: {folded}",
            test_case.description
        );
        assert_eq!(
            evaluated(&format!("CAST({folded} AS VARCHAR)")),
            evaluated(&format!("CAST({checked} AS VARCHAR)")),
            "{}",
            test_case.description
        );
    }
}
