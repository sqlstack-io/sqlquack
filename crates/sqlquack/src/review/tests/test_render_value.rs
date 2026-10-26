use sqlquack_compare::values::models::CanonicalValue;

use crate::review::_helpers::values::render_value;
use crate::review::tests::test_types::RenderValueTestCase;

#[test]
fn given_canonical_values_when_rendering_for_review_then_every_recorded_digit_is_shown() {
    let test_cases = [
        RenderValueTestCase {
            description: "a negative decimal below one keeps its leading zero",
            value: CanonicalValue::Number {
                unscaled: -7813,
                scale: 6,
            },
            declared_scale: Some(6),
            expected_text: "-0.007813",
        },
        RenderValueTestCase {
            description: "a decimal is padded back to the column's declared scale",
            value: CanonicalValue::Number {
                unscaled: 12,
                scale: 0,
            },
            declared_scale: Some(2),
            expected_text: "12.00",
        },
        RenderValueTestCase {
            description: "a timestamp before the epoch keeps its nanoseconds",
            value: CanonicalValue::Timestamp {
                nanos: -1,
                zone: None,
            },
            declared_scale: None,
            expected_text: "1969-12-31 23:59:59.999999999",
        },
        RenderValueTestCase {
            description: "a zoned timestamp names its zone",
            value: CanonicalValue::Timestamp {
                nanos: 1_709_207_101_123_456_789,
                zone: Some("UTC".to_owned()),
            },
            declared_scale: None,
            expected_text: "2024-02-29 11:45:01.123456789 UTC",
        },
        RenderValueTestCase {
            description: "a date far from the epoch",
            value: CanonicalValue::Date(-719_162),
            declared_scale: None,
            expected_text: "0001-01-01",
        },
        RenderValueTestCase {
            description: "text escapes tabs and marks the empty string",
            value: CanonicalValue::Text("\ta".to_owned()),
            declared_scale: None,
            expected_text: "\\ta",
        },
        RenderValueTestCase {
            description: "the empty string is marked",
            value: CanonicalValue::Text(String::new()),
            declared_scale: None,
            expected_text: "(empty)",
        },
        RenderValueTestCase {
            description: "NULL is spelled out",
            value: CanonicalValue::Null,
            declared_scale: None,
            expected_text: "NULL",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            render_value(&test_case.value, test_case.declared_scale),
            test_case.expected_text,
            "{}",
            test_case.description
        );
    }
}
