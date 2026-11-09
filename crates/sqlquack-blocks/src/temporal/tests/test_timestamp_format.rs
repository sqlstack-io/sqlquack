use crate::temporal::main::timestamp_format::timestamp_format;
use crate::temporal::tests::helpers::formatted;
use crate::temporal::tests::test_types::{FormattedTimestampTestCase, RejectedFormatTestCase};

#[test]
fn given_formatted_texts_when_reading_them_then_fields_spaces_and_mismatches_behave_as_recorded() {
    let test_cases = [
        FormattedTimestampTestCase {
            description: "unpadded fields read one digit",
            format: "YYYY-MM-DD HH24:MI",
            text: "2024-2-9 9:05",
            expected_timestamp: Ok(Some("2024-02-09 09:05:00")),
        },
        FormattedTimestampTestCase {
            description: "a format space reads a run of spaces",
            format: "YYYY-MM-DD HH24:MI",
            text: "2024-02-29  13:45",
            expected_timestamp: Ok(Some("2024-02-29 13:45:00")),
        },
        FormattedTimestampTestCase {
            description: "a format space reads no space",
            format: "YYYY-MM-DD HH12:MI AM",
            text: "2024-02-29 01:45pm",
            expected_timestamp: Ok(Some("2024-02-29 13:45:00")),
        },
        FormattedTimestampTestCase {
            description: "twelve AM is midnight",
            format: "YYYY-MM-DD HH12:MIAM",
            text: "2024-02-29 12:05AM",
            expected_timestamp: Ok(Some("2024-02-29 00:05:00")),
        },
        FormattedTimestampTestCase {
            description: "an hour outside 1-12 is NULL",
            format: "YYYY-MM-DD HH12:MI AM",
            text: "2024-02-29 13:45 PM",
            expected_timestamp: Ok(None),
        },
        FormattedTimestampTestCase {
            description: "an impossible date is NULL",
            format: "DD-MM-YYYY HH24:MI:SS",
            text: "31-04-2024 10:00:00",
            expected_timestamp: Ok(None),
        },
        FormattedTimestampTestCase {
            description: "a quoted literal reads itself, case-sensitively",
            format: "DD-MM-YYYY \"at\" HH24:MI",
            text: "29-02-2024 AT 13:45",
            expected_timestamp: Err(()),
        },
        FormattedTimestampTestCase {
            description: "adjacent numbers take two digits each",
            format: "YYYYMMDDHH24MISS",
            text: "20240229134501",
            expected_timestamp: Ok(Some("2024-02-29 13:45:01")),
        },
        FormattedTimestampTestCase {
            description: "adjacent numbers are not split unpadded",
            format: "YYYYMMDDHH24MISS",
            text: "202474957",
            expected_timestamp: Err(()),
        },
        FormattedTimestampTestCase {
            description: "empty text is NULL",
            format: "YYYY-MM-DD",
            text: "",
            expected_timestamp: Ok(None),
        },
    ];
    for test_case in test_cases {
        let actual = formatted(test_case.format, test_case.text);
        assert_eq!(
            actual,
            test_case
                .expected_timestamp
                .map(|value| value.map(str::to_owned)),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_unrecorded_formats_when_reading_them_then_they_are_rejected() {
    let test_cases = [
        RejectedFormatTestCase {
            description: "fractions have no recording",
            format: "YYYY-MM-DD HH24:MI:SS.FF3",
            expected_rejected: true,
        },
        RejectedFormatTestCase {
            description: "HH12 needs AM or PM",
            format: "YYYY-MM-DD HH12:MI",
            expected_rejected: true,
        },
        RejectedFormatTestCase {
            description: "a year is required",
            format: "MM-DD HH24:MI",
            expected_rejected: true,
        },
        RejectedFormatTestCase {
            description: "zone elements have no recording",
            format: "YYYY-MM-DD TZH",
            expected_rejected: true,
        },
        RejectedFormatTestCase {
            description: "a recorded format is read",
            format: "DD. MM. YYYY kl. HH24:MI",
            expected_rejected: false,
        },
    ];
    for test_case in test_cases {
        let rejected = timestamp_format(test_case.format).is_err();
        assert_eq!(
            rejected, test_case.expected_rejected,
            "{}",
            test_case.description
        );
    }
}
